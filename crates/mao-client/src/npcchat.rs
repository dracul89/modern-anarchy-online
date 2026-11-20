//! Talking to NPCs: GUI.dll's `NPCChatWindow_c` and its `NPCChatView_c` (FUN_100596c2), fed by the server's `Knubot`
//! messages.
//!
//! The view is, top to bottom, the conversation (a `TextView_c` with HTML, flags 0x6c: mouse input, selection, multiline,
//! word wrap; scrolling), the separator (`GFX_GUI_NPCCHAT_SEPERATOR`, 0x120), the answers (another such text view) and a
//! button bar (FUN_10059eee): Desc (0x3f, `N3Msg_NPCChatRequestDescription`, FUN_10058cdb), Info (0x44, the NPC's
//! `charid://` page, FUN_10058ad5), Give (0x42, `N3Msg_NPCChatStartTrade`, FUN_10058d16) and Shop (0x49: the handler at
//! 0x10058b34 is `N3Msg_UseItem` with the NPC and `false`, a plain use of the NPC, to which the server opens the NPC's
//! shop: the captures' Antonio Stacklund, Barry the Food Vendor and Marco Spida, each used a second or two after the
//! chat was opened, and the shop's list came at once). The server opens
//! the window (`KnubotOpenChatWindow`), appends lines (`KnubotAppendText`: the text and its kind) and gives the answers
//! (`KnubotAnswerList`); a clicked answer is told as the player's line and is `KnubotAnswer` with its place in the list
//! (FUN_10059504, the list emptied till the next), and closing the window is
//! `KnubotCloseChatWindow`. A conversation starts with the client's own `KnubotOpenChatWindow` to the NPC (as aosharp
//! does against the live servers).
//!
//! The lines' look is FUN_10058ee7's: by kind, 0 the NPC's text (`CCNPCChatText`, the NPC's name first when the speaker
//! changed), 1 out of character (`CCNPCOOCText`), 2 the player's question (`CCNPCChatQuestion`, the player's name first),
//! 3 system (`CCNPCChatSystem`), 4 an emote (`CCNPCChatEmote`, the NPC's name first), 5 a description
//! (`CCNPCChatDescription`); `\n` in the text is a line break. The answers are FUN_100595ae's: a bullet
//! (`GFX_GUI_NPCCHAT_BULLET`) and the escaped answer as a link to its number, in `CCNPCChatQuestion`.
//!
//! Giving (`NPCChatTradeBar_c`, FUN_10057d8b, in a window of its own with one tab, text.mdb 10000's "Trade", docked at
//! the chat window's right, its top at the chat window's top: FUN_10059236, `Window::DockWindow(2)`,
//! `LayoutDockedWindows` 0x101553b5): Give asks the NPC (`KnubotStartTrade`), and Give again with the bar up is its X
//! (FUN_10058d16). The NPC's answer (`KnubotStartTrade` with the slots and a text) puts the text in place of the answers,
//! in `CCNPCChatTrade`, and opens the bar: "GIVE ITEM" (one slot) or "GIVE ITEMS", an item grid of that many slots
//! (`ItemContainerView_c` FUN_100ce55d over the trade's own items, one column for one slot, else two), an inventory item
//! dropped on it going in while there is room (`KnubotTrade` action 0, FUN_10057adc) and taken out by a click (action 1,
//! FUN_10057c0b; with Shift its page, FUN_10057c5d), "GIVE CREDITS" over the credits field, and the V
//! (`GFX_GUI_BUTTON_V_*`) and X (`GFX_GUI_BUTTON_X_*`) buttons. V gives (`KnubotFinishTrade` accepted, the credits no more
//! than the character has: FUN_10057cc6) and leaves the bar up; X declines it and closes the bar (FUN_10058bf2,
//! FUN_10058aa1). Items the NPC does not want come back (`KnubotRejectedItems`: the inventory's, `net.rs`; the window
//! says nothing of them, as GUI.dll has no handler).

use crate::gui::host::Signal;
use crate::gui::multilist;
use crate::gui::view::{Id, Tree};

/// What the server said.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// The window opened, with the NPC's name.
    Open((u32, u32), String),
    Text((u32, u32), u32, String),
    Answers((u32, u32), Vec<String>),
    Close((u32, u32)),
    /// The NPC's trade: its slots and text.
    StartTrade((u32, u32), u32, String),
}

/// What the window asks the network thread to send.
#[derive(Clone, Debug, PartialEq)]
pub enum Command {
    Answer((u32, u32), u32),
    Describe((u32, u32)),
    Close((u32, u32)),
    /// Give: ask for the NPC's trade.
    StartTrade((u32, u32)),
    /// An inventory place put in (true) or taken out.
    TradeItem((u32, u32), u32, bool),
    /// The trade ended: the credits, accepted or declined.
    FinishTrade((u32, u32), u32, bool),
    /// Shop: the NPC is used, and the server opens its shop when it keeps one.
    Shop((u32, u32)),
}

/// The NPC's trade bar while it is open (`NPCChatTradeBar_c`): its slots, the inventory places put in, and its views
/// (the tree, its root and the item grid), the rows the grid last showed, and when the right button went down.
#[derive(Default)]
pub struct Trade {
    pub slots: u32,
    /// The inventory places put in.
    pub items: Vec<u32>,
    bar: Option<(Tree, Id, Id)>,
    shown: Vec<multilist::Item>,
    right_since: Option<std::time::Instant>,
    /// A test script's `give@` (an inventory place put in, as a drop on the bar does) and `giveaccept@` (V pressed).
    pub script_give: Option<u32>,
    pub script_accept: bool,
}

/// The bar's views as FUN_10057d8b builds them, top to bottom in a `BorderView_c`: "GIVE ITEM(S)" (font 5, 5 round it),
/// the item grid's place, "GIVE CREDITS" (5 round it but below), the credits field in a border view (5 round it but
/// below; the field 3 in, as wide as there is room), and V (5 round it) and X (5 round it but left) side by side. The
/// field is the original's editable `TextView_c` (feature flags 0x200d); here the view system's text input.
const BAR: &str = r#"<BorderView name="NPCChatTradeBar" view_layout="vertical">
  <TextView name="GiveItems" layout_borders="Rect(5,5,5,5)" value=""/>
  <View name="ItemDock"/>
  <TextView name="GiveCredits" layout_borders="Rect(5,5,5,0)" value="GIVE CREDITS"/>
  <BorderView layout_borders="Rect(5,5,5,0)">
    <TextInputView name="Credits" layout_borders="Rect(3,3,3,3)" value=""/>
  </BorderView>
  <View view_layout="horizontal">
    <Button name="Accept" layout_borders="Rect(5,5,5,5)"/>
    <Button name="Decline" layout_borders="Rect(0,5,5,5)"/>
  </View>
</BorderView>"#;

/// The bar's label over its slots (FUN_10057d8b): the program's own English words.
pub fn give_label(slots: u32) -> &'static str {
    if slots == 1 { "GIVE ITEM" } else { "GIVE ITEMS" }
}

/// The item grid's cells across and down (FUN_10057d8b's `SetViewCellCounts`, the same for the fewest and the most): one
/// column for one slot, else two, and the rows half the slots, rounded up.
pub fn bar_cells(slots: u32) -> (usize, usize) {
    (usize::from(slots != 1) + 1, (slots as usize).div_ceil(2))
}

/// The C library's `atoi`: spaces, a sign, then digits as far as they go (wrapping as the 32-bit sum does).
fn atoi(text: &str) -> i32 {
    let t = text.trim_start_matches(|c: char| matches!(c as u32, 9..=13 | 32));
    let (negative, digits) = match t.as_bytes().first() {
        Some(b'-') => (true, &t[1..]),
        Some(b'+') => (false, &t[1..]),
        _ => (false, t),
    };
    let v = digits.bytes().take_while(u8::is_ascii_digit).fold(0i32, |v, d| v.wrapping_mul(10).wrapping_add(i32::from(d - b'0')));
    if negative { v.wrapping_neg() } else { v }
}

/// FUN_10057cc6: the credits V gives: the field's number (`atoi`), and no more than the character's credits (stat 61),
/// compared as unsigned numbers (a negative number is more than any).
pub fn credits_given(typed: &str, own: i64) -> u32 {
    let (asked, own) = (atoi(typed) as u32, own as u32);
    if own < asked { own } else { asked }
}

/// The view (FUN_100596c2's layout) as the skins' views are written.
const VIEW: &str = r#"<View name="NPCChat" view_layout="vertical" layout_borders="Rect(3,3,3,3)">
  <TextView name="Text" feature_flags="TVF_ACCEPT_MOUSE_INPUT|TVF_ALLOW_TEXT_SELECTION|TVF_MULTILINE|TVF_WORD_WRAP" min_size="Point(320,170)" max_size="Point(16000,16000)" layout_borders="Rect(5,5,5,0)"/>
  <BitmapView name="Separator" bitmap_id="GFX_GUI_NPCCHAT_SEPERATOR" max_size="Point(16000,-1)"/>
  <TextView name="Answers" feature_flags="TVF_ACCEPT_MOUSE_INPUT|TVF_ALLOW_TEXT_SELECTION|TVF_MULTILINE|TVF_WORD_WRAP" min_size="Point(320,90)" max_size="Point(16000,16000)" layout_borders="Rect(5,5,5,5)"/>
  <View name="Buttons" view_layout="horizontal" layout_borders="Rect(5,2,5,2)">
    <Button name="Desc" tooltip="Describe"/>
    <Button name="Info" tooltip="Info"/>
    <Button name="Give" tooltip="Give"/>
    <Button name="Shop" tooltip="Shop"/>
    <HLayoutSpacer/>
  </View>
</View>"#;

/// The buttons' pictures (FUN_10059eee: normal, pressed).
const BUTTONS: [(&str, &str, &str); 4] = [
    ("Desc", "GFX_GUI_BUTTON_DESC_NORMAL", "GFX_GUI_BUTTON_DESC_PRESSED"),
    ("Info", "GFX_GUI_BUTTON_INFO_NORMAL", "GFX_GUI_BUTTON_INFO_PRESSED"),
    ("Give", "GFX_GUI_BUTTON_GIVE_NORMAL", "GFX_GUI_BUTTON_GIVE_PRESSED"),
    ("Shop", "GFX_GUI_BUTTON_SHOP_NORMAL", "GFX_GUI_BUTTON_SHOP_PRESSED"),
];

#[derive(Default)]
pub struct NpcChatWindow {
    pub open: bool,
    pub npc: (u32, u32),
    /// The NPC's name (the window's title and the lines') and the player's.
    pub name: String,
    pub own_name: String,
    /// The conversation's markup, the kind of its last line, and the answers.
    text: String,
    last_kind: Option<u32>,
    pub answers: Vec<String>,
    /// The info page asked for (`charid://`), until taken.
    pub info_url: Option<String>,
    pub trade: Option<Trade>,
    /// The NPC's text for the trade, shown in the answers' place until the next answers (FUN_10059236's `SetText`).
    trade_note: Option<String>,
    /// The character's credits (stat 61), told by the window's owner: what V gives at most.
    pub credits: i64,
    tree: Option<(Tree, Id)>,
    /// Scroll the conversation to its end at the next drawing (and the one after).
    to_end: bool,
    to_end_next: bool,
}

/// `String::Escape` of `<>&"`.
fn escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// FUN_10058ee7: one line of `kind` in the conversation's markup (`last` the kind before it, `empty` whether there is
/// no text yet).
pub fn line(kind: u32, text: &str, npc: &str, own: &str, last: Option<u32>, empty: bool) -> String {
    let mut out = String::new();
    match kind {
        1 => out.push_str("<br><font color=CCNPCOOCText>"),
        2 => {
            out.push_str("<br><font color=CCNPCChatQuestion>");
            out.push_str(&escape(own));
            out.push_str(": ");
        }
        3 => out.push_str("<br><font color=CCNPCChatSystem>"),
        4 => {
            out.push_str("<br><font color=CCNPCChatEmote>");
            out.push_str(&escape(npc));
            out.push(' ');
        }
        5 => out.push_str("<br><font color=CCNPCChatDescription>"),
        _ => {
            out.push_str("<font color=CCNPCChatText>");
            // (the speaker's name when the NPC starts speaking again)
            if last != Some(0) {
                if !empty {
                    out.push_str("<br>");
                }
                out.push_str(&escape(npc));
                out.push_str(": ");
            }
        }
    }
    out.push_str(&text.replace("\\n", "<br>").replace('\n', "<br>"));
    out.push_str("</font>");
    out
}

/// FUN_100595ae: the answers' markup.
pub fn answers(list: &[String]) -> String {
    list.iter()
        .enumerate()
        .map(|(i, a)| format!("<div indent=wrapped><img src=tdb://id:GFX_GUI_NPCCHAT_BULLET> <a href={i} style=text-decoration:none><font color=CCNPCChatQuestion>{}</font></a></div>", escape(a)))
        .collect()
}

impl NpcChatWindow {
    /// What the server said; the commands it calls for.
    pub fn apply(&mut self, event: Event) {
        match event {
            Event::Open(npc, name) => {
                if !self.open || self.npc != npc {
                    self.text.clear();
                    self.last_kind = None;
                    self.answers.clear();
                    self.trade_note = None;
                }
                self.npc = npc;
                self.name = if name.is_empty() { "NPC".into() } else { name };
                self.open = true;
            }
            Event::Text(npc, kind, text) => {
                if !self.open || self.npc != npc {
                    self.apply(Event::Open(npc, String::new()));
                }
                let l = line(kind, &text, &self.name, &self.own_name, self.last_kind, self.text.is_empty());
                self.text.push_str(&l);
                self.last_kind = Some(kind);
                self.to_end = true;
            }
            Event::Answers(npc, list) => {
                if !self.open || self.npc != npc {
                    self.apply(Event::Open(npc, String::new()));
                }
                self.answers = list;
                self.trade_note = None;
            }
            Event::Close(npc) => {
                if npc == self.npc {
                    self.open = false;
                    self.trade = None;
                }
            }
            Event::StartTrade(npc, slots, message) => {
                if !self.open || self.npc != npc {
                    self.apply(Event::Open(npc, String::new()));
                }
                // (FUN_10059236: only while no bar is up; the NPC's text in the answers' place)
                if self.trade.is_none() {
                    self.trade = Some(Trade { slots, ..Default::default() });
                    self.trade_note = Some(format!("<font color=CCNPCChatTrade>{message}</font>"));
                }
            }
        }
    }

    fn load(&mut self, ctx: &egui::Context) {
        let doc = crate::gui::xml::parse(VIEW);
        let mut tree = Tree::default();
        tree.measure = crate::gui::font::Measure { ctx: Some(ctx.clone()) };
        let Some(root) = doc.first().and_then(|e| crate::gui::create(&mut tree, e)) else { return };
        for (name, normal, pressed) in BUTTONS {
            if let Some(id) = tree.find(root, name)
                && let Some(b) = tree.control_mut::<crate::gui::controls::Button>(id)
            {
                b.pictures = Some([normal.to_string(), pressed.to_string()]);
            }
        }
        // (Shop is a button like the others: until 2026-11-18 it was switched off here, "not known", and a shopkeeper
        // who also talks, as Arete Landing's all do, could not be bought from at all)
        self.tree = Some((tree, root));
    }

    /// Draws the window (and the trade bar, `items` the inventory's); the commands to send.
    pub fn show(&mut self, ctx: &egui::Context, palette: &crate::theme::Palette, items: &[crate::gear::GearItem], gear: &mut crate::gear::GearWindow) -> Vec<Command> {
        let mut commands = Vec::new();
        if !self.open {
            return commands;
        }
        if self.tree.is_none() {
            self.load(ctx);
        }
        let Some((tree, root)) = self.tree.as_mut() else { return commands };
        let root = *root;
        let set = |tree: &mut Tree, name: &str, text: &str| {
            if let Some(id) = tree.find(root, name)
                && let Some(t) = tree.control_mut::<crate::gui::controls::TextView>(id)
                && t.text != text
            {
                let scroll = t.scroll;
                t.set_text(text);
                t.scroll = scroll;
                tree.relayout = true;
            }
        };
        set(tree, "Text", &self.text);
        set(tree, "Answers", &self.trade_note.clone().unwrap_or_else(|| answers(&self.answers)));
        let skin = crate::skin::skin(ctx);
        let min = crate::gui::host::preferred_pixels(tree, root, false);
        let mut open = self.open;
        let shown = crate::chrome::window(ctx, palette, "NPCChat", &self.name, &mut open, |w| w.default_size(min.max(egui::vec2(340.0, 320.0))).resizable(true), |ui| {
            crate::gui::host::show(ui, tree, root, skin.as_deref(), palette);
        });
        // (the trade bar's window, docked at the chat window's right, its top at the chat window's: slot 2 of
        // `LayoutDockedWindows`; it goes when its X closes it)
        if let (Some(t), Some(chat)) = (self.trade.as_mut(), shown.map(|r| r.response.rect))
            && trade_window(ctx, palette, t, self.npc, items, gear, chat.right_top(), self.credits, &mut self.info_url, &mut commands)
        {
            self.trade = None;
        }
        // (the conversation follows its newest line, once it has been laid out at the view's width)
        if std::mem::take(&mut self.to_end)
            && let Some(id) = tree.find(root, "Text")
        {
            let height = tree.get(id).frame.bounds().height() + 1.0;
            if let Some(t) = tree.control_mut::<crate::gui::controls::TextView>(id) {
                t.scroll_to_end(height);
            }
            // (again next frame: this one measured the text before its new lines)
            self.to_end_next = true;
        } else if std::mem::take(&mut self.to_end_next)
            && let Some(id) = tree.find(root, "Text")
        {
            let height = tree.get(id).frame.bounds().height() + 1.0;
            if let Some(t) = tree.control_mut::<crate::gui::controls::TextView>(id) {
                t.scroll_to_end(height);
            }
        }
        for (id, signal) in std::mem::take(&mut tree.signals) {
            match (tree.get(id).name.as_str(), signal) {
                // (FUN_10059504: an answer in the list is told as the player's line (FUN_10058ee7 kind 2), sent, and the
                // answers taken away until the NPC gives new ones; a number past the list does nothing)
                ("Answers", Signal::Link(href)) => {
                    if let Ok(k) = href.trim().parse::<u32>()
                        && let Some(a) = self.answers.get(k as usize).cloned()
                    {
                        let l = line(2, &escape(&a), &self.name, &self.own_name, self.last_kind, self.text.is_empty());
                        self.text.push_str(&l);
                        self.last_kind = Some(2);
                        self.to_end = true;
                        commands.push(Command::Answer(self.npc, k));
                        self.answers.clear();
                    }
                }
                ("Desc", Signal::Clicked) => commands.push(Command::Describe(self.npc)),
                ("Info", Signal::Clicked) => self.info_url = Some(format!("charid://{}/{}", self.npc.0, self.npc.1)),
                // (FUN_10058d16: Give asks for a trade; with the bar up it is the bar's X)
                ("Give", Signal::Clicked) => {
                    if self.trade.take().is_some() {
                        commands.push(Command::FinishTrade(self.npc, 0, false));
                    } else {
                        commands.push(Command::StartTrade(self.npc));
                    }
                }
                ("Shop", Signal::Clicked) => commands.push(Command::Shop(self.npc)),
                _ => {}
            }
        }
        if !open {
            self.open = false;
            if self.trade.take().is_some() {
                commands.push(Command::FinishTrade(self.npc, 0, false));
            }
            commands.push(Command::Close(self.npc));
        }
        commands
    }
}

/// FUN_10057d8b: the bar's views ([`BAR`]) with the label for its slots, V's and X's pictures (0x4b/0x4c and 0x4d/0x4e),
/// and in its place the item grid: the trade's own items (`ItemContainerView_c` of kind 2 over the character, column
/// flags 0xb, list flags 0xe), sorted by name and arranged (FUN_100ce55d), as many slot pictures as the slots
/// (`SetMaxItemCount`), no scroll bars, and room for [`bar_cells`] cells, no more and no fewer.
fn build_bar(ctx: &egui::Context, slots: u32) -> Option<(Tree, Id, Id)> {
    let doc = crate::gui::xml::parse(BAR);
    let mut tree = Tree::default();
    tree.measure = crate::gui::font::Measure { ctx: Some(ctx.clone()) };
    let root = doc.first().and_then(|e| crate::gui::create(&mut tree, e))?;
    if let Some(id) = tree.find(root, "GiveItems")
        && let Some(t) = tree.control_mut::<crate::gui::controls::TextView>(id)
    {
        t.set_text(give_label(slots));
    }
    for (name, normal, pressed) in [("Accept", "GFX_GUI_BUTTON_V_NORMAL", "GFX_GUI_BUTTON_V_PRESSED"), ("Decline", "GFX_GUI_BUTTON_X_NORMAL", "GFX_GUI_BUTTON_X_PRESSED")] {
        if let Some(id) = tree.find(root, name)
            && let Some(b) = tree.control_mut::<crate::gui::controls::Button>(id)
        {
            b.pictures = Some([normal.to_string(), pressed.to_string()]);
        }
    }
    let dock = tree.find(root, "ItemDock")?;
    let (across, down) = bar_cells(slots);
    let list = multilist::create_grid(&mut tree, "ItemContainerView", slots as i32, across);
    for (id, label, width, flags) in crate::shop::columns(0xb) {
        multilist::add_column(&mut tree, list, id, label, width, flags);
    }
    multilist::set_sort(&mut tree, list, 1, 0);
    multilist::set_scroll_modes(&mut tree, list, 0, 0);
    multilist::set_view_cells(&mut tree, list, (across, down), (across, down));
    multilist::click_only(&mut tree, list);
    tree.get_mut(dock).layout = crate::gui::view::Layout::Base;
    tree.add_child(dock, list);
    Some((tree, root, list))
}

/// The bar's window at `at` (FUN_10059236: a `Window` of one tab, text.mdb 10000's "Trade", holding the bar); what was
/// done in it goes to `commands`, an item's page asked for to `info`. True when the bar goes: its X, or its window
/// closed (the client's own: the frame's close is taken for the X).
#[allow(clippy::too_many_arguments)]
fn trade_window(ctx: &egui::Context, palette: &crate::theme::Palette, t: &mut Trade, npc: (u32, u32), items: &[crate::gear::GearItem], gear: &mut crate::gear::GearWindow, at: egui::Pos2, own_credits: i64, info: &mut Option<String>, commands: &mut Vec<Command>) -> bool {
    if t.bar.is_none() {
        t.bar = build_bar(ctx, t.slots);
    }
    if ctx.input(|i| i.pointer.button_pressed(egui::PointerButton::Secondary)) {
        t.right_since = Some(std::time::Instant::now());
    }
    let right_held = t.right_since.map_or(0.0, |s| s.elapsed().as_secs_f32());
    let Some((tree, root, list)) = t.bar.as_mut() else { return false };
    let (root, list) = (*root, *list);
    // (the grid's rows: the items put in, by their places)
    let rows: Vec<multilist::Item> = t.items.iter().filter_map(|slot| items.iter().find(|i| i.slot == *slot)).map(|i| crate::shop::row(i.slot as i64, i.icon.as_ref().map(|c| gear.texture(ctx, c)), &i.name, i.count as u32, i.quality, None)).collect();
    if t.shown != rows {
        multilist::set_items(tree, list, rows.clone());
        t.shown = rows;
    }
    multilist::update(tree, list);
    let skin = crate::skin::skin(ctx);
    let min = crate::gui::host::preferred_pixels(tree, root, false);
    let dragged = crate::gear::DRAGGED.load(std::sync::atomic::Ordering::Relaxed);
    let shift = ctx.input(|i| i.modifiers.shift);
    let (mut open, mut right_act, mut tab) = (true, None, 0usize);
    let label = crate::gui::xml::ldb_named(10000, "Trade", "Trade");
    let (put, slots) = (t.items.clone(), t.slots);
    let mut added = t.script_give.take().filter(|slot| !put.contains(slot) && (put.len() as u32) < slots);
    crate::chrome::window(ctx, palette, "NPCChatTrade", "", &mut open, |w| w.current_pos(at).movable(false).resizable(false).default_size(min), |ui| {
        crate::chrome::tab_view(ui, palette, &[label.as_str()], &mut tab, |ui, _| {
            let response = crate::gui::host::show(ui, tree, root, skin.as_deref(), palette);
            let origin = response.rect.min;
            let pointer = ui.input(|i| i.pointer.hover_pos());
            let hovered = pointer.and_then(|p| multilist::item_at(tree, list, origin, p));
            // (an inventory item let go on the grid goes in while it holds fewer than the slots; not a split, FUN_10057adc)
            let area = crate::gui::host::screen_rect(tree, list, origin);
            if dragged != u32::MAX
                && !put.contains(&dragged)
                && put.len() < slots as usize
                && crate::gear::split_count() == 0
                && crate::gear::letting_go(ui.ctx())
                && ui.input(|i| i.pointer.interact_pos().is_some_and(|p| area.contains(p)))
            {
                added = Some(dragged);
            }
            // (a right click on an item is the list's click, as a left one: FUN_1004141b; held for the menu's delay,
            // nothing)
            if let Some(slot) = hovered
                && ui.input(|i| i.pointer.secondary_clicked())
                && crate::shop::row_click(true, shift, right_held) == crate::shop::RowClick::Act
            {
                right_act = Some(slot);
            }
            if let Some(name) = hovered.and_then(|slot| items.iter().find(|i| i.slot as i64 == slot)).map(|i| i.name.clone()) {
                response.on_hover_text(name);
            }
        });
    });
    if let Some(slot) = added {
        t.items.push(slot);
        commands.push(Command::TradeItem(npc, slot, true));
    }
    // (a click on an item takes it out, FUN_10057c0b; with Shift it shows its page, FUN_10057c5d)
    let mut clicked: Vec<(i64, bool)> = multilist::take_events(tree).into_iter().filter_map(|(_, e)| if let multilist::ItemEvent::Clicked(slot, _) = e { Some((slot, shift)) } else { None }).collect();
    clicked.extend(right_act.map(|slot| (slot, false)));
    let mut close = !open;
    for (slot, shift) in clicked {
        let slot = slot as u32;
        match crate::shop::row_click(false, shift, 0.0) {
            crate::shop::RowClick::Page => {
                if let Some(i) = items.iter().find(|i| i.slot == slot) {
                    *info = Some(format!("itemref://{}/{}/{}", i.item_id, i.high_id, i.quality));
                }
            }
            _ => {
                if t.items.contains(&slot) {
                    t.items.retain(|s| *s != slot);
                    commands.push(Command::TradeItem(npc, slot, false));
                }
            }
        }
    }
    let field = tree.find(root, "Credits");
    for (id, signal) in std::mem::take(&mut tree.signals) {
        match (tree.get(id).name.as_str(), signal) {
            // (V: the field's credits, no more than one has, written back, and the trade given; the bar stays)
            ("Accept", Signal::Clicked) => {
                let typed = field.and_then(|f| crate::gui::textinput::text(tree, f)).unwrap_or("").to_string();
                let credits = credits_given(&typed, own_credits);
                if credits != atoi(&typed) as u32
                    && let Some(f) = field
                {
                    crate::gui::textinput::set_text(tree, f, &credits.to_string());
                }
                commands.push(Command::FinishTrade(npc, credits, true));
            }
            ("Decline", Signal::Clicked) => close = true,
            _ => {}
        }
    }
    if std::mem::take(&mut t.script_accept) {
        commands.push(Command::FinishTrade(npc, 0, true));
    }
    // (X: declined, and the bar goes)
    if close {
        commands.push(Command::FinishTrade(npc, 0, false));
    }
    close
}

/// The server's `Knubot` messages to the window (`kind`, `data` past the identity): none for others.
pub fn event(kind: u32, data: &[u8]) -> Option<Event> {
    let p = mao_net::schema::Schema::global().parse_n3(kind, data).ok()?;
    let v = &p.value;
    let target = v.get("Target")?;
    let int = |v: &mao_net::schema::Value, f: &str| v.get(f).and_then(mao_net::schema::Value::as_int).unwrap_or(0);
    let npc = (int(target, "Type") as u32, int(target, "Instance") as u32);
    let text = |v: &mao_net::schema::Value| match v.get("Text") {
        Some(mao_net::schema::Value::Str(s)) => s.clone(),
        Some(mao_net::schema::Value::Bytes(b)) => String::from_utf8_lossy(b).into_owned(),
        _ => String::new(),
    };
    match kind {
        0x3b132d64 => Some(Event::Open(npc, String::new())),
        0x5d70532a => Some(Event::Text(npc, int(v, "Unknown3") as u32, text(v))),
        0x55704d31 => match v.get("DialogOptions") {
            Some(mao_net::schema::Value::List(l)) => Some(Event::Answers(npc, l.iter().map(text).collect())),
            _ => Some(Event::Answers(npc, Vec::new())),
        },
        0x270a4c62 => Some(Event::Close(npc)),
        0x7864401d => Some(Event::StartTrade(npc, int(v, "NumberOfItemSlotsInTradeWindow") as u32, match v.get("Message") {
            Some(mao_net::schema::Value::Str(s)) => s.clone(),
            Some(mao_net::schema::Value::Bytes(b)) => String::from_utf8_lossy(b).into_owned(),
            _ => String::new(),
        })),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_and_answers_are_the_originals() {
        assert_eq!(line(0, "Hello\\nthere", "Bob", "Me", None, true), "<font color=CCNPCChatText>Bob: Hello<br>there</font>");
        assert_eq!(line(0, "More", "Bob", "Me", Some(0), false), "<font color=CCNPCChatText>More</font>");
        assert_eq!(line(2, "Yes", "Bob", "Me", Some(0), false), "<br><font color=CCNPCChatQuestion>Me: Yes</font>");
        assert_eq!(line(0, "Good", "Bob", "Me", Some(2), false), "<font color=CCNPCChatText><br>Bob: Good</font>");
        assert_eq!(answers(&["a<b".into()]), "<div indent=wrapped><img src=tdb://id:GFX_GUI_NPCCHAT_BULLET> <a href=0 style=text-decoration:none><font color=CCNPCChatQuestion>a&lt;b</font></a></div>");
    }

    /// WIN-030: FUN_10057d8b's label and grid (one column for one slot, else two; half the slots' rows), V's credits no
    /// more than one has (FUN_10057cc6, `atoi` compared unsigned), the NPC's text in the answers' place and a second
    /// trade's offer ignored while the bar is up (FUN_10059236).
    #[test]
    fn the_trade_bar_is_the_originals() {
        assert_eq!((give_label(1), give_label(3)), ("GIVE ITEM", "GIVE ITEMS"));
        assert_eq!([1, 2, 3, 5].map(bar_cells), [(1, 1), (2, 1), (2, 2), (2, 3)]);
        assert_eq!(credits_given("500", 1000), 500);
        assert_eq!(credits_given("5000", 1000), 1000);
        assert_eq!(credits_given("-5", 1000), 1000);
        assert_eq!((credits_given(" 12abc", 100), credits_given("", 100), credits_given("+7", 100)), (12, 0, 7));
        let mut w = NpcChatWindow::default();
        let npc = (0xc350, 7);
        w.apply(Event::Answers(npc, vec!["Yes".into()]));
        w.apply(Event::StartTrade(npc, 3, "Give me three.".into()));
        assert_eq!(w.trade.as_ref().map(|t| t.slots), Some(3));
        assert_eq!(w.trade_note.as_deref(), Some("<font color=CCNPCChatTrade>Give me three.</font>"));
        w.trade.as_mut().unwrap().items.push(70);
        w.apply(Event::StartTrade(npc, 1, "Again.".into()));
        assert_eq!(w.trade.as_ref().map(|t| (t.slots, t.items.len())), Some((3, 1)));
        // (the next answers take the text's place)
        w.apply(Event::Answers(npc, vec!["Thanks".into()]));
        assert!(w.trade_note.is_none());
        // (the grid holds room for its cells exactly, no bars: two across and two down for three slots)
        let ctx = egui::Context::default();
        let (mut tree, root, list) = build_bar(&ctx, 3).unwrap();
        multilist::update(&mut tree, list);
        assert_eq!((tree.get(list).min_size, tree.get(list).max_limit), (crate::gui::geom::Point::new(117.0, 117.0), crate::gui::geom::Point::new(117.0, 117.0)));
        assert!(tree.find(root, "Credits").is_some() && tree.find(root, "Accept").is_some() && tree.find(root, "Decline").is_some());
    }

    #[test]
    fn the_servers_messages_are_read() {
        // (AppendText: header byte, i16, the NPC, i32, the text with its i32 length; nothing follows the text in any capture)
        let mut d = vec![0u8, 0, 2];
        for w in [0xc350u32, 0x1234, 0] {
            d.extend_from_slice(&w.to_be_bytes());
        }
        d.extend_from_slice(&5u32.to_be_bytes());
        d.extend_from_slice(b"Hello");
        assert_eq!(event(0x5d70532a, &d), Some(Event::Text((0xc350, 0x1234), 0, "Hello".into())));
        // (the 2026-11-08 capture's first line at the agency, as sent)
        let line = "0000020000c3507b22e1970000000000000037506f69736f6e6f7573732c20737461747573202d20756e61737369676e65642e20576861742063616e204920646f20666f7220796f753f";
        let d: Vec<u8> = (0..line.len() / 2).map(|i| u8::from_str_radix(&line[2 * i..2 * i + 2], 16).unwrap()).collect();
        assert!(matches!(event(0x5d70532a, &d), Some(Event::Text((0xc350, 0x7b22e197), 0, t)) if t.ends_with("What can I do for you?")));
    }
}
