//! Talking to NPCs: GUI.dll's `NPCChatWindow_c` and its `NPCChatView_c` (FUN_100596c2), fed by the server's `Knubot`
//! messages.
//!
//! The view is, top to bottom, the conversation (a `TextView_c` with HTML, flags 0x6c: mouse input, selection, multiline,
//! word wrap; scrolling), the separator (`GFX_GUI_NPCCHAT_SEPERATOR`, 0x120), the answers (another such text view) and a
//! button bar (FUN_10059eee): Desc (0x3f, `N3Msg_NPCChatRequestDescription`, FUN_10058cdb), Info (0x44, the NPC's
//! `charid://` page, FUN_10058ad5), Give (0x42, `N3Msg_NPCChatStartTrade`, FUN_10058d16) and Shop (0x49). The server opens
//! the window (`KnubotOpenChatWindow`), appends lines (`KnubotAppendText`: the text and its kind) and gives the answers
//! (`KnubotAnswerList`); a clicked answer is `KnubotAnswer` with its place in the list, and closing the window is
//! `KnubotCloseChatWindow`. A conversation starts with the client's own `KnubotOpenChatWindow` to the NPC (as aosharp
//! does against the live servers).
//!
//! The lines' look is FUN_10058ee7's: by kind, 0 the NPC's text (`CCNPCChatText`, the NPC's name first when the speaker
//! changed), 1 out of character (`CCNPCOOCText`), 2 the player's question (`CCNPCChatQuestion`, the player's name first),
//! 3 system (`CCNPCChatSystem`), 4 an emote (`CCNPCChatEmote`, the NPC's name first), 5 a description
//! (`CCNPCChatDescription`); `\n` in the text is a line break. The answers are FUN_100595ae's: a bullet
//! (`GFX_GUI_NPCCHAT_BULLET`) and the escaped answer as a link to its number, in `CCNPCChatQuestion`.

use crate::gui::host::Signal;
use crate::gui::view::{Id, Tree};

/// What the server said.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// The window opened, with the NPC's name.
    Open((u32, u32), String),
    Text((u32, u32), u32, String),
    Answers((u32, u32), Vec<String>),
    Close((u32, u32)),
}

/// What the window asks the network thread to send.
#[derive(Clone, Debug, PartialEq)]
pub enum Command {
    Answer((u32, u32), u32),
    Describe((u32, u32)),
    Close((u32, u32)),
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
            }
            Event::Close(npc) => {
                if npc == self.npc {
                    self.open = false;
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
        // (Give and Shop start the NPC trade, which is not done yet)
        for name in ["Give", "Shop"] {
            if let Some(id) = tree.find(root, name) {
                tree.get_mut(id).enabled = false;
            }
        }
        self.tree = Some((tree, root));
    }

    /// Draws the window; the commands to send.
    pub fn show(&mut self, ctx: &egui::Context, palette: &crate::theme::Palette) -> Vec<Command> {
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
        set(tree, "Answers", &answers(&self.answers));
        let skin = crate::skin::skin(ctx);
        let min = crate::gui::host::preferred_pixels(tree, root, false);
        let mut open = self.open;
        crate::chrome::window(ctx, palette, "NPCChat", &self.name, &mut open, |w| w.default_size(min.max(egui::vec2(340.0, 320.0))).resizable(true), |ui| {
            crate::gui::host::show(ui, tree, root, skin.as_deref(), palette);
        });
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
                ("Answers", Signal::Link(href)) => {
                    if let Ok(k) = href.trim().parse::<u32>() {
                        if let Some(a) = self.answers.get(k as usize).cloned() {
                            let l = line(2, &escape(&a), &self.name, &self.own_name, self.last_kind, self.text.is_empty());
                            self.text.push_str(&l);
                            self.last_kind = Some(2);
                            self.to_end = true;
                        }
                        commands.push(Command::Answer(self.npc, k));
                    }
                }
                ("Desc", Signal::Clicked) => commands.push(Command::Describe(self.npc)),
                ("Info", Signal::Clicked) => self.info_url = Some(format!("charid://{}/{}", self.npc.0, self.npc.1)),
                _ => {}
            }
        }
        if !open {
            self.open = false;
            commands.push(Command::Close(self.npc));
        }
        commands
    }
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

    #[test]
    fn the_servers_messages_are_read() {
        // (AppendText: header byte, i16, the NPC, i32, the text with its i32 length, the kind)
        let mut d = vec![0u8, 0, 2];
        for w in [0xc350u32, 0x1234, 0] {
            d.extend_from_slice(&w.to_be_bytes());
        }
        d.extend_from_slice(&5u32.to_be_bytes());
        d.extend_from_slice(b"Hello");
        d.extend_from_slice(&0u32.to_be_bytes());
        assert_eq!(event(0x5d70532a, &d), Some(Event::Text((0xc350, 0x1234), 0, "Hello".into())));
    }
}
