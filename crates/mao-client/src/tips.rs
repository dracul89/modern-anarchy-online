//! Tips (GUI.dll's `TipSystem_t`, `TipView_c` and `TipButtonWindow_c`): the game's help pages that open when the player
//! first does something.
//!
//! `TipSystem_t` reads `cd_image/text/tips/TipHeader.xml` (`LoadIndexFile` 0x10053f68: each `<Tip event= data= file=>`)
//! and queues the events other code reports (`TriggerEvent` 0x100262ef, here [`trigger`]). Each frame
//! (`SlotFrameProcess` 0x1005464a), while tips are on (`IsTipEnabled`), it counts the minutes played (an
//! `AfterMinutesOfPlay` event each new minute) and matches the queued events against the header: a tip of the event whose
//! `data` matches the event's (FUN_100539be: none, `x`, `x*`, `*x` or `*x*`, without regard to case) and which is not
//! switched off opens the tip window (`tip_window`) and is added to its list (FUN_10056f47, unless it is the one shown).
//!
//! The window (`TipView_c` FUN_10057096 on `Views/TipView.xml`) shows a tip's file (FUN_10056acd): its `<title>` as the
//! window's title and its `<section>`s, each as a line after a bullet, up to the number opened so far, with "Next
//! Section... (n/m)" while there are more, and "Previous Tip" / "Next Tip" when the list has more than one. "Do not show
//! this tip again" switches the shown tip off (FUN_100568bf: `EnableTip`). Closing it (FUN_100568ef) drops the tips
//! switched off from the list, asks whether to disable all tips if that box is checked (`DisableTips` 0x10053b06), and
//! while tips remain leaves the "new tip" button (`TipButtonWindow_c` FUN_100565d7: `GFX_GUI_NEW_TIP`, near the lower
//! right corner) that opens the window again.
//!
//! The original keeps the switched-off tips per character (`DisabledTipsMap.xml`) and `IsTipEnabled` with the
//! preferences; here both are in `tips.ini` beside the settings.

use crate::gui::host::Signal;
use crate::theme::Palette;
use std::collections::HashSet;
use std::sync::Mutex;

/// The events reported and not yet matched (`TipSystem_t`'s queue, +0xa8).
static PENDING: Mutex<Vec<(String, String)>> = Mutex::new(Vec::new());

/// `TipSystem_t::TriggerEvent`: something happened that may have a tip (`data` the item, playfield, profession ...).
pub fn trigger(event: &str, data: &str) {
    PENDING.lock().unwrap_or_else(|e| e.into_inner()).push((event.to_string(), data.to_string()));
}

/// One `<Tip>` of the header.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tip {
    pub event: String,
    pub data: String,
    pub file: String,
}

impl Tip {
    /// The tip's id (what is switched off): its event and file.
    pub fn key(&self) -> String {
        format!("{}{}", self.event, self.file)
    }
}

/// The header's tips.
pub fn parse_header(text: &str) -> Vec<Tip> {
    fn all(e: &crate::gui::xml::Element, out: &mut Vec<Tip>) {
        if e.name == "Tip"
            && let (Some(event), Some(file)) = (e.attribute("event"), e.attribute("file"))
        {
            out.push(Tip { event: event.to_string(), data: e.attribute("data").unwrap_or("").to_string(), file: file.to_string() });
        }
        for c in &e.children {
            all(c, out);
        }
    }
    let mut out = Vec::new();
    for e in crate::gui::xml::parse(text) {
        all(&e, &mut out);
    }
    out
}

/// FUN_100539be: whether an event's data matches a tip's pattern (empty: all; `*x*` holds x; `*x` ends in x; `x*`
/// begins with it; else the same), without regard to case. An event without data matches every tip of the event.
pub fn matches(pattern: &str, data: &str) -> bool {
    if pattern.is_empty() || data.is_empty() {
        return true;
    }
    let (p, d) = (pattern.to_lowercase(), data.to_lowercase());
    match (p.strip_prefix('*'), p.strip_suffix('*')) {
        (Some(_), Some(_)) if p.len() >= 3 => d.contains(&p[1..p.len() - 1]),
        (Some(end), _) => d.ends_with(end),
        (None, Some(start)) => d.starts_with(start),
        (None, None) => d == p,
    }
}

/// A tip file: its title and its sections' markup.
pub fn parse_file(text: &str) -> (String, Vec<String>) {
    let title = text.find("<title>").and_then(|a| text[a + 7..].find("</title>").map(|b| text[a + 7..a + 7 + b].trim().to_string())).unwrap_or_default();
    let mut sections = Vec::new();
    let mut rest = text;
    while let Some(a) = rest.find("<section>") {
        let body = &rest[a + 9..];
        let Some(b) = body.find("</section>") else { break };
        sections.push(body[..b].to_string());
        rest = &body[b + 10..];
    }
    (title, sections)
}

/// FUN_10056acd: the window's text for a tip with `opened` sections showing, and where in the list it is.
pub fn page(sections: &[String], opened: usize, at: usize, count: usize) -> String {
    let mut s = String::new();
    for section in sections.iter().take(opened) {
        s += "<div><font color=CCInfoHeader>o</font><font color=CCInfoHeadline> ";
        s += section;
        s += "</font></div>";
    }
    if opened < sections.len() {
        s += &format!("<a href=\"NextSubTip\">Next Section... ({}/{})</a>", opened, sections.len());
    }
    if count >= 2 {
        s += "<br>";
        if at != 0 {
            s += "<a href=\"PrevTip\">Previous Tip</a> ";
        }
        if at + 1 != count {
            s += "<a href=\"NextTip\">Next Tip</a>";
        }
    }
    s
}

/// A tip in the window's list: its file, id and the sections opened.
struct Shown {
    file: String,
    key: String,
    opened: usize,
}

#[derive(Default)]
pub struct Tips {
    loaded: bool,
    header: Vec<Tip>,
    /// `IsTipEnabled`.
    pub enabled: bool,
    disabled: HashSet<String>,
    list: Vec<Shown>,
    current: usize,
    window: Option<crate::gui::window::XmlWindow>,
    /// The tip shown in the window (its index), once its text is in.
    filled: Option<usize>,
    /// The "new tip" button (`TipButtonWindow_c`).
    button: bool,
    /// "Are you sure you want to disable all tips?"
    confirm: bool,
    /// Play time (`TipSystem_t` +0xa0) and the minutes reported (+0xa4).
    played: f32,
    minutes: i32,
    /// The window to open (`tip_window` set).
    wanted: bool,
    /// What [`Tips::watch`] saw last: the playfield, the inventory, skills and shop windows open.
    seen: (String, bool, bool, bool),
}

fn path() -> std::path::PathBuf {
    crate::settings::Settings::path().with_file_name("tips.ini")
}

fn save_file(enabled: bool, disabled: &HashSet<String>) {
    let mut text = format!("enabled={}\n", u8::from(enabled));
    let mut keys: Vec<&String> = disabled.iter().collect();
    keys.sort();
    for k in keys {
        text += &format!("disabled={k}\n");
    }
    let _ = std::fs::write(path(), text);
}

fn game() -> Option<std::path::PathBuf> {
    crate::theme::game_dir().map(|g| g.join("cd_image/text/tips"))
}

impl Tips {
    /// Closes the tip window as its close button does (Escape); whether it was shown.
    pub fn close(&mut self) -> bool {
        match self.window.as_mut() {
            Some(w) if w.open => {
                w.open = false;
                true
            }
            _ => false,
        }
    }

    fn load(&mut self) {
        self.loaded = true;
        self.enabled = true;
        if let Some(dir) = game() {
            self.header = std::fs::read_to_string(dir.join("TipHeader.xml")).map(|t| parse_header(&t)).unwrap_or_default();
        }
        for line in std::fs::read_to_string(path()).unwrap_or_default().lines() {
            match line.split_once('=') {
                Some(("enabled", v)) => self.enabled = v.trim() != "0",
                Some(("disabled", k)) => {
                    self.disabled.insert(k.trim().to_string());
                }
                _ => {}
            }
        }
    }

    fn save(&self) {
        save_file(self.enabled, &self.disabled);
    }

    /// `SlotFrameProcess`: the minutes played and the events since the last frame.
    fn process(&mut self, dt: f32) {
        let events = std::mem::take(&mut *PENDING.lock().unwrap_or_else(|e| e.into_inner()));
        if !self.enabled {
            return;
        }
        self.played += dt;
        let minutes = (self.played / 60.0) as i32;
        let mut events = events;
        if self.minutes < minutes {
            self.minutes = minutes;
            events.push(("AfterMinutesOfPlay".into(), minutes.to_string()));
        }
        for (event, data) in events {
            let found: Vec<Tip> = self.header.iter().filter(|t| t.event == event && matches(&t.data, &data) && !self.disabled.contains(&t.key())).cloned().collect();
            for tip in found {
                // (FUN_10056f47: added unless it is the one shown)
                let key = tip.key();
                if self.list.get(self.current).is_some_and(|s| s.key == key) {
                    continue;
                }
                self.list.push(Shown { file: tip.file, key, opened: 1 });
                self.current = self.list.len() - 1;
                self.filled = None;
                self.button = false;
                self.wanted = true;
            }
        }
    }

    /// FUN_10056acd: the shown tip into the window.
    fn fill(&mut self) {
        let Some(w) = self.window.as_mut() else { return };
        let Some(shown) = self.list.get(self.current) else { return };
        let text = game().and_then(|d| std::fs::read(d.join(&shown.file)).ok()).map(|b| String::from_utf8_lossy(&b).into_owned());
        let body = match &text {
            Some(text) => {
                let (title, sections) = parse_file(text);
                w.title = title;
                page(&sections, shown.opened, self.current, self.list.len())
            }
            None => "Failed to load Tip File".to_string(),
        };
        if let Some(id) = w.tree.find(w.root, "BrowserView")
            && let Some(t) = w.tree.control_mut::<crate::gui::controls::TextView>(id)
        {
            t.set_text(&body);
        }
        if let Some(id) = w.tree.find(w.root, "DisableOneTipCheckBox") {
            crate::gui::controls::CheckBox::set(&mut w.tree, id, self.disabled.contains(&shown.key));
        }
        w.tree.relayout = true;
        self.filled = Some(self.current);
    }

    /// The events the interface's state shows: a new playfield (its name with `_` for spaces, as the header's
    /// `Arrival_hall`), and the inventory, skills (with the profession's name) and shop windows opening.
    pub fn watch(&mut self, playfield: &str, inventory: bool, skills: bool, shop: bool, profession: i64) {
        const PROFESSIONS: [&str; 16] = ["", "Soldier", "Martial Artist", "Engineer", "Fixer", "Agent", "Adventurer", "Trader", "Bureaucrat", "Enforcer", "Doctor", "Nano-Technician", "MetaPhysicist", "", "Keeper", "Shade"];
        if !playfield.is_empty() && playfield != self.seen.0 {
            self.seen.0 = playfield.to_string();
            trigger("OnEnteringPlayfield", &playfield.replace(' ', "_"));
        }
        let opened = |was: &mut bool, now: bool| std::mem::replace(was, now) != now && now;
        if opened(&mut self.seen.1, inventory) {
            trigger("OnOpenInventoryView", "");
        }
        if opened(&mut self.seen.2, skills) {
            trigger("OnOpenSkillView", PROFESSIONS.get(profession as usize).copied().unwrap_or(""));
        }
        if opened(&mut self.seen.3, shop) {
            trigger("OnOpeningShop", "");
        }
    }

    /// The window, the "new tip" button and the question; links to other pages are returned for the info view.
    pub fn show(&mut self, ctx: &egui::Context, palette: &Palette, dt: f32) -> Vec<String> {
        if !self.loaded {
            self.load();
        }
        self.process(dt);
        if std::mem::take(&mut self.wanted) && self.window.is_none() {
            self.window = crate::gui::window::XmlWindow::load(ctx, "tip_window", "Tips", "TipView.xml");
            if let Some(w) = self.window.as_mut() {
                w.tree.measure = crate::gui::font::Measure { ctx: Some(ctx.clone()) };
                w.default_size = egui::vec2(360.0, 260.0);
            }
        }
        let mut urls = Vec::new();
        if self.window.is_some() && self.filled != Some(self.current) {
            self.fill();
        }
        if let Some(w) = self.window.as_mut() {
            let signals = w.show(ctx, palette);
            let mut refill = false;
            let mut save = false;
            for (id, signal) in signals {
                let parent = w.tree.get(id).parent.map(|p| w.tree.get(p).name.clone()).unwrap_or_default();
                match signal {
                    Signal::Link(url) => match url.as_str() {
                        "NextSubTip" => {
                            if let Some(s) = self.list.get_mut(self.current) {
                                s.opened += 1;
                                refill = true;
                            }
                        }
                        "NextTip" if self.current + 1 < self.list.len() => {
                            self.current += 1;
                            refill = true;
                        }
                        "PrevTip" if self.current > 0 => {
                            self.current -= 1;
                            refill = true;
                        }
                        "NextTip" | "PrevTip" => {}
                        _ => urls.push(url),
                    },
                    Signal::Toggled(on) if parent == "DisableOneTipCheckBox" => {
                        if let Some(s) = self.list.get(self.current) {
                            if on {
                                self.disabled.insert(s.key.clone());
                            } else {
                                self.disabled.remove(&s.key);
                            }
                            save = true;
                        }
                    }
                    _ => {}
                }
            }
            if refill {
                self.filled = None;
            }
            if save {
                save_file(self.enabled, &self.disabled);
            }
            if !w.open {
                // FUN_100568ef: the tips switched off leave the list; "Disable tips" asks; the button while tips remain
                let all = w.tree.find(w.root, "DisableTipsCheckBox").is_some_and(|id| crate::gui::controls::CheckBox::checked(&w.tree, id));
                self.window = None;
                self.list.retain(|s| !self.disabled.contains(&s.key));
                self.current = self.list.len().saturating_sub(1);
                self.filled = None;
                if all {
                    self.confirm = true;
                }
                self.button = !self.list.is_empty() && self.enabled;
            }
        }
        if self.confirm {
            let mut open = true;
            let question = crate::gui::xml::ldb_text("#10000:212410675");
            let mut answer = None;
            crate::chrome::window(ctx, palette, "disable-tips", &crate::gui::xml::ldb_text("#DisableTips"), &mut open, |w| w.resizable(false).anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO), |ui| {
                ui.label(egui::RichText::new(&question).color(crate::chrome::rgb(palette.text)));
                ui.horizontal(|ui| {
                    if crate::chrome::button(ui, palette, "Yes", true).clicked() {
                        answer = Some(true);
                    }
                    if crate::chrome::button(ui, palette, "No", true).clicked() {
                        answer = Some(false);
                    }
                });
            });
            if answer == Some(true) {
                // (`SlotDisableTipsDlg`: `IsTipEnabled` off, which also takes the button away)
                self.enabled = false;
                self.button = false;
                self.save();
            }
            if answer.is_some() || !open {
                self.confirm = false;
            }
        }
        if self.button {
            self.new_tip_button(ctx);
        }
        urls
    }

    /// `TipButtonWindow_c`: `GFX_GUI_NEW_TIP` near the lower right; a click opens the tip window again.
    fn new_tip_button(&mut self, ctx: &egui::Context) {
        let skin = crate::skin::skin(ctx);
        let size = skin.as_ref().and_then(|s| s.get("GFX_GUI_NEW_TIP")).map(|s| s.size).unwrap_or(egui::vec2(32.0, 32.0));
        let mut clicked = false;
        egui::Area::new(egui::Id::unique("new-tip")).anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-60.0, -60.0)).order(egui::Order::Foreground).show(ctx, |ui| {
            let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
            let drawn = skin.as_ref().is_some_and(|s| s.draw_in(ui.painter(), "GFX_GUI_NEW_TIP", rect, egui::Color32::WHITE));
            if !drawn {
                ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, "?", egui::FontId::proportional(20.0), egui::Color32::YELLOW);
            }
            clicked = response.on_hover_text("Tips").clicked();
        });
        if clicked {
            self.button = false;
            self.wanted = true;
            self.filled = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn patterns() {
        assert!(matches("", "anything"));
        assert!(matches("Guard", ""));
        assert!(matches("guard", "Guard"));
        assert!(!matches("Guard", "Guardian"));
        assert!(matches("Remains*", "Remains of Leet"));
        assert!(matches("*Training", "Arete Training"));
        assert!(matches("*Startup Crystal - Quick Heal", "Doctor: Startup Crystal - Quick Heal"));
        assert!(matches("*Basic*", "a basic board"));
        assert!(!matches("*Basic*", "a board"));
    }

    #[test]
    fn a_page() {
        let (title, sections) = parse_file("<title>You have been invited!</title>\n<section>one</section>\n<section>two</section>");
        assert_eq!(title, "You have been invited!");
        assert_eq!(sections, ["one", "two"]);
        let p = page(&sections, 1, 0, 1);
        assert_eq!(p, "<div><font color=CCInfoHeader>o</font><font color=CCInfoHeadline> one</font></div><a href=\"NextSubTip\">Next Section... (1/2)</a>");
        let p = page(&sections, 2, 1, 3);
        assert!(p.ends_with("</div><br><a href=\"PrevTip\">Previous Tip</a> <a href=\"NextTip\">Next Tip</a>"));
    }

    #[test]
    fn the_games_header() {
        let Some(game) = crate::theme::game_dir() else { return };
        let Ok(text) = std::fs::read_to_string(game.join("cd_image/text/tips/TipHeader.xml")) else { return };
        let tips = parse_header(&text);
        assert!(tips.len() > 80, "{}", tips.len());
        assert!(tips.iter().any(|t| t.event == "OnInvitedToTeam" && t.file == "InvitedToATeam.html" && t.data.is_empty()));
        assert!(tips.iter().any(|t| t.event == "AfterMinutesOfPlay" && t.data == "10"));
    }
}
