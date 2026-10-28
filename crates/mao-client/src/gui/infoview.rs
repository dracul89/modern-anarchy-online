//! `InfoView_c` (GUI.dll, made by FUN_100ef289): the "Info" window (`info_window`), `Views/InfoView.xml`'s text view
//! `BrowserView` with `BackButton` and `ForwardButton`, showing pages by URL with a history.
//!
//! The URLs (FUN_100ee973): `text://<markup>` the markup itself; `file://<path>[?...]` a text file (the path as it is,
//! else under the game's `cd_image/text/help/`); `chatcmd://<command>` runs a chat command; `itemref://<low>/<high>/<ql>`,
//! `itemid://...` and `skillid://<n>` are pages the game builds (N3's item and skill descriptions): they are handed to
//! the game's side ([`InfoView::take_requests`]) and come back through [`InfoView::show_page`]. A URL's
//! `?section=<n>` shows only the n-th `<section>` of its text (FUN_100ee39b). A new page drops the pages after the
//! current one; showing the current URL again replaces its text.

use super::controls::TextView;
use super::host::Signal;
use super::window::XmlWindow;

struct Page {
    url: String,
    text: String,
    scroll: f32,
}

#[derive(Default)]
pub struct InfoView {
    pub open: bool,
    window: Option<XmlWindow>,
    pages: Vec<Page>,
    at: usize,
    requests: Vec<String>,
    commands: Vec<String>,
}

/// FUN_100ee39b's `?section=<n>`: the n-th `<section>`...`</section>` of the text (counted from 0), or all of it.
pub fn section(url: &str, text: &str) -> String {
    let Some(query) = url.split_once('?').map(|q| q.1) else { return text.to_string() };
    let mut wanted: Option<usize> = None;
    for pair in query.split('&') {
        if let Some((k, v)) = pair.split_once('=') {
            if k.trim().eq_ignore_ascii_case("section") {
                wanted = v.trim().parse().ok();
            }
        }
    }
    let Some(n) = wanted else { return text.to_string() };
    let lower = text.to_ascii_lowercase();
    let mut from = 0;
    for k in 0..=n {
        let Some(i) = lower[from..].find("<section").map(|i| i + from) else { return text.to_string() };
        let Some(open_end) = lower[i..].find('>').map(|e| e + i + 1) else { return text.to_string() };
        if k == n {
            let end = lower[open_end..].find("</section").map_or(text.len(), |e| e + open_end);
            return text[open_end..end].to_string();
        }
        from = open_end;
    }
    text.to_string()
}

impl InfoView {
    /// `InfoViewModule_c::ShowURL`: opens the window on a URL.
    pub fn show_url(&mut self, url: &str) {
        self.open = true;
        let lower = url.to_ascii_lowercase();
        if let Some(markup) = url.strip_prefix("text://") {
            // (the history keys a text page by the sum of its bytes)
            let key: u32 = markup.bytes().map(|b| b as i8 as i32 as u32).fold(0u32, u32::wrapping_add);
            self.show_page(&format!("text://{key}"), markup);
        } else if lower.starts_with("file://") {
            let path = url[7..].split('?').next().unwrap_or("").to_string();
            let text = std::fs::read(&path)
                .ok()
                .or_else(|| crate::theme::game_dir().and_then(|g| std::fs::read(g.join("cd_image/text/help").join(&path)).ok()))
                .or_else(|| crate::theme::game_dir().and_then(|g| std::fs::read(g.join("cd_image/text").join(&path)).ok()));
            match text {
                Some(t) => self.show_page(url, &String::from_utf8_lossy(&t)),
                None => self.show_page(url, &format!("Infoview failed to load and show file: <{path}>")),
            }
        } else if lower.starts_with("chatcmd://") {
            self.commands.push(url[10..].to_string());
        } else if lower.starts_with("itemref://") || lower.starts_with("itemid://") || lower.starts_with("skillid://") || lower.starts_with("shopitemid://") {
            self.requests.push(url.to_string());
        }
    }

    /// `InfoViewModule_c::ShowText`: opens the window on markup.
    pub fn show_text(&mut self, markup: &str) {
        self.show_url(&format!("text://{markup}"));
    }

    /// FUN_100ee39b: a page into the history and the window.
    pub fn show_page(&mut self, url: &str, text: &str) {
        self.open = true;
        let text = section(url, text);
        if let Some(current) = self.pages.get_mut(self.at).filter(|p| p.url == url) {
            current.text = text;
            current.scroll = 0.0;
        } else {
            self.save_scroll();
            if !self.pages.is_empty() {
                self.pages.truncate(self.at + 1);
            }
            self.pages.push(Page { url: url.to_string(), text, scroll: 0.0 });
            self.at = self.pages.len() - 1;
        }
        self.refresh();
    }

    /// The pages the game is to build (`itemref://` and the like), since the last call.
    pub fn take_requests(&mut self) -> Vec<String> {
        std::mem::take(&mut self.requests)
    }

    /// The chat commands its `chatcmd://` links asked for.
    pub fn take_commands(&mut self) -> Vec<String> {
        std::mem::take(&mut self.commands)
    }

    fn browser(&mut self) -> Option<&mut TextView> {
        let w = self.window.as_mut()?;
        let id = w.tree.find(w.root, "BrowserView")?;
        w.tree.control_mut::<TextView>(id)
    }

    fn save_scroll(&mut self) {
        let scroll = self.browser().map(|b| b.scroll);
        if let (Some(s), Some(p)) = (scroll, self.pages.get_mut(self.at)) {
            p.scroll = s;
        }
    }

    /// The text view and buttons for the current page (FUN_100edf3e / FUN_100ee06e's enabling).
    fn refresh(&mut self) {
        let (text, scroll) = match self.pages.get(self.at) {
            Some(p) => (p.text.clone(), p.scroll),
            None => return,
        };
        if let Some(b) = self.browser() {
            b.set_text(&text);
            b.scroll = scroll;
        }
        let (back, forward) = (self.at > 0, self.at + 1 < self.pages.len());
        if let Some(w) = self.window.as_mut() {
            for (name, on) in [("BackButton", back), ("ForwardButton", forward)] {
                if let Some(id) = w.tree.find(w.root, name) {
                    w.tree.get_mut(id).enabled = on;
                }
            }
        }
    }

    pub fn show(&mut self, ctx: &egui::Context, palette: &crate::theme::Palette) {
        if !self.open {
            return;
        }
        if self.window.is_none() {
            self.window = XmlWindow::load(ctx, "Info", "Info", "InfoView.xml");
            self.refresh();
        }
        let Some(w) = self.window.as_mut() else { return };
        w.open = true;
        let signals = w.show(ctx, palette);
        self.open = w.open;
        for (id, s) in signals {
            let name = self.window.as_ref().map(|w| w.tree.get(id).name.clone()).unwrap_or_default();
            match s {
                Signal::Clicked if name == "BackButton" && self.at > 0 => {
                    self.save_scroll();
                    self.at -= 1;
                    self.refresh();
                }
                Signal::Clicked if name == "ForwardButton" && self.at + 1 < self.pages.len() => {
                    self.save_scroll();
                    self.at += 1;
                    self.refresh();
                }
                Signal::Link(url) => self.show_url(&url),
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn history_drops_the_forward_pages() {
        let mut v = InfoView::default();
        v.show_page("a", "1");
        v.show_page("b", "2");
        v.at = 0;
        v.show_page("c", "3");
        assert_eq!(v.pages.iter().map(|p| p.url.as_str()).collect::<Vec<_>>(), ["a", "c"]);
        v.show_page("c", "4");
        assert_eq!(v.pages.len(), 2);
        assert_eq!(v.pages[1].text, "4");
    }

    #[test]
    fn a_section_of_a_page() {
        let text = "x<section>zero</section><section a=1>one</section>";
        assert_eq!(section("file://a?section=1", text), "one");
        assert_eq!(section("file://a", text), text);
    }
}
