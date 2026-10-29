//! The in-game web pages: GUI.dll's `BrowserWindow_c` (its `Initialize` 0x1010fdb8), an Awesomium page there and a WebView2
//! page here (wry), laid over the game window where the window's frame leaves room for it.
//!
//! The original's modes: 1 the item store, **2 the market** (the market terminal: its use runs the spell 0xcf7b "open a client
//! window" with window 0, Gamecode FUN_100a9cf4 → GUI.dll FUN_1002e2fe → `BrowserWindow_c(2)`), 3 customer service, 4 the daily
//! rewards. Each has a home page (`GetHomeURL` 0x1010fc05) and sends the game's own headers to its site's pages (a header
//! definition "Anarchy" and a rule for the site, `AddHTTPHeaderDef` / `AddHTTPHeaderRule`):
//!
//! | header | value |
//! |---|---|
//! | `X-Anarchy-CharacterID` | the character (`InputConfig_t`'s +0xdc) |
//! | `X-Anarchy-PlayerID` | the account (`Client_t::s_nPlayerID`, from the login ticket) |
//! | `X-Anarchy-DimensionType` | `Client_t::s_nEventServerType` (the ticket's) |
//! | `X-Anarchy-Cookie` | `s_nCookie1 s_nCookie2` (the ticket's key) |
//! | `X-Anarchy-ServerID` | the server the messages come from (`Client_t`'s +0xac) |
//! | `X-Anarchy-Appearance` | stats 59, 4, 47 and 64 |
//! | `X-Platform` | `Native` (`Steam` when `RunningSteam`) |
//!
//! The page sees a script object `AnarchyOnline` (`RegisterJSObject`): `AcceptLanguage` (the locale), `IsInGame` (true) and the
//! callback `OpenMarketSendWindow` (`SlotOpenMarketSendWindow` 0x1010fa77: the deposit window), which comes back here as a
//! message ([`Browser::take_events`]).

use crate::theme::Palette;

/// Which pages a window shows (`BrowserMode_e`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// 1: the item store.
    ItemStore,
    /// 2: the market.
    Market,
}

impl Mode {
    /// The window's title (`Initialize`: "Market").
    pub fn title(self) -> &'static str {
        match self {
            Mode::ItemStore => "Item Store",
            Mode::Market => "Market",
        }
    }

    /// `GetHomeURL`.
    pub fn home(self) -> &'static str {
        match self {
            Mode::ItemStore => "http://aoshop.funcom.com/shop/",
            Mode::Market => "http://aomarket.funcom.com/market/",
        }
    }

    /// The pages the headers go to (`AddHTTPHeaderRule`'s pattern, without its `*`).
    pub fn site(self) -> &'static str {
        match self {
            Mode::ItemStore => "http://aoshop.funcom.com/",
            Mode::Market => "http://aomarket.funcom.com/",
        }
    }
}

/// A page to open, with the headers its site is sent.
#[derive(Clone, Debug, PartialEq)]
pub struct Request {
    pub mode: Mode,
    pub headers: Vec<(String, String)>,
}

/// The game's headers (the table above): `character` and the stats are the character's, `server` the game server's id.
pub fn headers(character: u32, ticket: Option<&mao_net::system::Ticket>, server: u32, stat: &dyn Fn(u32) -> i64) -> Vec<(String, String)> {
    let (cookie1, cookie2) = ticket.map_or((0, 0), |t| t.cookies());
    let (event_server, player) = ticket.and_then(|t| t.account()).unwrap_or((0, 0));
    let u = |v: i64| (v as u32).to_string();
    vec![
        ("X-Anarchy-CharacterID".into(), character.to_string()),
        ("X-Anarchy-PlayerID".into(), player.to_string()),
        ("X-Anarchy-DimensionType".into(), event_server.to_string()),
        ("X-Anarchy-Cookie".into(), format!("{cookie1} {cookie2}")),
        ("X-Anarchy-ServerID".into(), server.to_string()),
        ("X-Anarchy-Appearance".into(), format!("{} {} {} {}", u(stat(59)), u(stat(4)), u(stat(47)), u(stat(64)))),
        ("X-Platform".into(), "Native".into()),
    ]
}

/// What a page script asks for (the `AnarchyOnline` object's callbacks).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    /// `OpenMarketSendWindow`.
    OpenMarketSendWindow,
}

/// What the window's buttons ask of the page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Navigate {
    Home,
    Back,
    Forward,
}

/// The window: its frame, its buttons and where the page goes (the page itself is the viewer's, [`Page`]).
#[derive(Default)]
pub struct Browser {
    pub open: bool,
    pub request: Option<Request>,
    /// Where the page goes, in screen points, as the window was last drawn (none while it is closed).
    pub area: Option<egui::Rect>,
    pub navigate: Vec<Navigate>,
    /// The page's address and whether it is loading (the original's URL bar and `StatusBar`).
    pub status: String,
    /// A new page was asked for: the old one goes.
    pub fresh: bool,
    events: std::sync::Arc<std::sync::Mutex<Vec<Event>>>,
}

impl Browser {
    /// Opens the window on a mode's home page.
    pub fn show_page(&mut self, request: Request) {
        self.request = Some(request);
        self.open = true;
        self.fresh = true;
    }

    /// Where the page's script callbacks go.
    pub fn event_sink(&self) -> std::sync::Arc<std::sync::Mutex<Vec<Event>>> {
        self.events.clone()
    }

    /// The script callbacks since the last call.
    pub fn take_events(&mut self) -> Vec<Event> {
        self.events.lock().map(|mut e| std::mem::take(&mut *e)).unwrap_or_default()
    }

    /// Draws the window's frame: Home, Back and Forward (`HomeButton`, `BackButton`, `ForwardButton`), the page's room and
    /// the status line.
    pub fn show(&mut self, ctx: &egui::Context, palette: &Palette) {
        self.area = None;
        let Some(mode) = self.request.as_ref().map(|r| r.mode) else { return };
        if !self.open {
            return;
        }
        let mut open = self.open;
        let mut area = None;
        let mut navigate = Vec::new();
        let status = self.status.clone();
        crate::chrome::window(ctx, palette, "Browser", mode.title(), &mut open, |w| w.default_size(egui::vec2(820.0, 620.0)).min_size(egui::vec2(400.0, 300.0)).resizable(true), |ui| {
            ui.horizontal(|ui| {
                for (label, what) in [("Home", Navigate::Home), ("<", Navigate::Back), (">", Navigate::Forward)] {
                    if crate::chrome::button(ui, palette, label, true).clicked() {
                        navigate.push(what);
                    }
                }
                ui.label(egui::RichText::new(&status).small().weak());
            });
            let size = ui.available_size().max(egui::vec2(200.0, 150.0));
            let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
            area = Some(rect);
        });
        self.open = open;
        self.area = area.filter(|_| open);
        self.navigate.extend(navigate);
    }
}

/// The script the page gets before its own: the `AnarchyOnline` object.
pub fn page_script(language: &str) -> String {
    format!(
        "window.AnarchyOnline = {{ AcceptLanguage: {language:?}, IsInGame: true, \
         OpenMarketSendWindow: function() {{ window.ipc.postMessage('OpenMarketSendWindow'); }} }};"
    )
}

/// The page loads through this protocol: on Windows `aomarket://localhost/<path>` is served as
/// `http://aomarket.localhost/<path>`, and every request to it is passed on to the site with the game's headers
/// ([`forward`]): what Awesomium's header rule does for the original. (WebView2's own request hook is a COM call this
/// client's `unsafe`-free code can't make.)
pub const PROXY_SCHEME: &str = "aomarket";
pub const PROXY_ORIGIN: &str = "http://aomarket.localhost";

/// The site's address of a request to the proxy (either way WebView2 names it).
pub fn site_url(site: &str, uri: &str) -> String {
    let path = [PROXY_ORIGIN, "aomarket://localhost"].iter().find_map(|p| uri.strip_prefix(p)).unwrap_or(uri);
    format!("{}{}", site.trim_end_matches('/'), if path.is_empty() { "/" } else { path })
}

/// The proxy's address of a page on the site (a link or a redirect to it).
pub fn proxy_url(site: &str, url: &str) -> Option<String> {
    url.strip_prefix(site.trim_end_matches('/')).map(|rest| format!("{PROXY_ORIGIN}{rest}"))
}

/// A page's text with the site's own addresses made the proxy's (also as JSON writes them, `http:\/\/`).
pub fn rewrite_text(site: &str, text: &str) -> String {
    let origin = site.trim_end_matches('/');
    text.replace(origin, PROXY_ORIGIN).replace(&origin.replace('/', "\\/"), &PROXY_ORIGIN.replace('/', "\\/"))
}

/// A cookie the site sets, for the proxy's host: without its `Domain` (the browser would refuse funcom.com's for localhost).
pub fn rewrite_cookie(cookie: &str) -> String {
    cookie.split(';').filter(|part| !part.trim_start().to_ascii_lowercase().starts_with("domain=")).collect::<Vec<_>>().join(";")
}

/// The script that keeps the page's own requests on the proxy: `fetch` and `XMLHttpRequest` to the site's address go to
/// the proxy instead.
pub fn proxy_script(site: &str) -> String {
    let origin = site.trim_end_matches('/');
    format!(
        "(function(){{var R={origin:?},P={PROXY_ORIGIN:?};function f(u){{return (typeof u==='string'&&u.indexOf(R)===0)?P+u.slice(R.length):u;}}\
         if(window.fetch){{var of=window.fetch;window.fetch=function(i,o){{return of.call(this,f(i),o);}};}}\
         var oo=XMLHttpRequest.prototype.open;XMLHttpRequest.prototype.open=function(m,u){{arguments[1]=f(u);return oo.apply(this,arguments);}};}})();"
    )
}

/// One request to the proxy passed on to the site with the game's headers; the answer with the site's addresses made the
/// proxy's. Runs on its own thread (the protocol answers asynchronously).
#[cfg(windows)]
fn forward(site: &str, game_headers: &[(String, String)], request: wry::http::Request<Vec<u8>>) -> wry::http::Response<Vec<u8>> {
    use std::io::Read;
    let url = site_url(site, &request.uri().to_string());
    let agent = ureq::AgentBuilder::new().redirects(0).timeout(std::time::Duration::from_secs(30)).build();
    let mut out = agent.request(request.method().as_str(), &url);
    // (what the page sends that the site needs: its cookies, what it posts, what it takes; the addresses made the site's)
    for (name, value) in request.headers() {
        let name = name.as_str();
        if ["cookie", "content-type", "accept", "accept-language", "user-agent", "x-requested-with"].contains(&name) {
            if let Ok(v) = value.to_str() {
                out = out.set(name, v);
            }
        } else if name == "referer" {
            if let Ok(v) = value.to_str() {
                out = out.set(name, &site_url(site, v));
            }
        }
    }
    for (name, value) in game_headers {
        out = out.set(name, value);
    }
    let body = request.into_body();
    let answer = if body.is_empty() { out.call() } else { out.send_bytes(&body) };
    let answer = match answer {
        Ok(a) | Err(ureq::Error::Status(_, a)) => a,
        Err(e) => {
            let text = format!("<html><body style='font-family:sans-serif'><h3>The market could not be reached</h3><p>{e}</p></body></html>");
            return wry::http::Response::builder().status(502).header("Content-Type", "text/html").body(text.into_bytes()).unwrap_or_default();
        }
    };
    let mut response = wry::http::Response::builder().status(answer.status());
    let kind = answer.content_type().to_ascii_lowercase();
    for name in answer.headers_names() {
        let lower = name.to_ascii_lowercase();
        if ["content-length", "transfer-encoding", "connection", "content-encoding"].contains(&lower.as_str()) {
            continue;
        }
        for value in answer.all(&name) {
            let value = match lower.as_str() {
                "set-cookie" => rewrite_cookie(value),
                "location" => proxy_url(site, value).unwrap_or_else(|| value.to_string()),
                _ => value.to_string(),
            };
            response = response.header(name.as_str(), value);
        }
    }
    let mut bytes = Vec::new();
    let _ = answer.into_reader().take(64 << 20).read_to_end(&mut bytes);
    if kind.starts_with("text/") || kind.contains("javascript") || kind.contains("json") {
        bytes = rewrite_text(site, &String::from_utf8_lossy(&bytes)).into_bytes();
    }
    response.body(bytes).unwrap_or_default()
}

/// The page: a WebView2 child of the game window, its site reached through the proxy.
#[cfg(windows)]
pub struct Page {
    view: wry::WebView,
    _context: wry::WebContext,
    bounds: (i32, i32, u32, u32),
    mode: Mode,
    /// Pages of the site the page went to directly (a link with its full address), to load through the proxy instead.
    redirected: std::sync::Arc<std::sync::Mutex<Vec<String>>>,
}

#[cfg(windows)]
impl Page {
    /// Opens a request's home page over `window` at `bounds` (physical pixels: x, y, width, height). The page's data (its
    /// cookies and cache) are kept in `data` (the original's `/Browser` folder).
    pub fn open(window: &winit::window::Window, request: &Request, bounds: (i32, i32, u32, u32), data: std::path::PathBuf, events: std::sync::Arc<std::sync::Mutex<Vec<Event>>>) -> Result<Page, String> {
        let mut context = wry::WebContext::new(Some(data));
        let site = request.mode.site().to_string();
        let redirected: std::sync::Arc<std::sync::Mutex<Vec<String>>> = Default::default();
        let (proxy_site, game_headers) = (site.clone(), std::sync::Arc::new(request.headers.clone()));
        let (nav_site, nav_queue) = (site.clone(), redirected.clone());
        let home = proxy_url(&site, request.mode.home()).unwrap_or_else(|| request.mode.home().to_string());
        let view = wry::WebViewBuilder::new_with_web_context(&mut context)
            .with_bounds(rect(bounds))
            .with_initialization_script(page_script("en"))
            .with_initialization_script(proxy_script(&site))
            .with_asynchronous_custom_protocol(PROXY_SCHEME.into(), move |_, request, responder| {
                let (site, headers) = (proxy_site.clone(), game_headers.clone());
                std::thread::spawn(move || responder.respond(forward(&site, &headers, request)));
            })
            .with_navigation_handler(move |url| match proxy_url(&nav_site, &url) {
                Some(proxied) => {
                    if let Ok(mut q) = nav_queue.lock() {
                        q.push(proxied);
                    }
                    false
                }
                None => true,
            })
            .with_ipc_handler(move |message: wry::http::Request<String>| {
                if message.body() == "OpenMarketSendWindow" {
                    if let Ok(mut e) = events.lock() {
                        e.push(Event::OpenMarketSendWindow);
                    }
                }
            })
            .with_url(home)
            .build_as_child(window)
            .map_err(|e| e.to_string())?;
        Ok(Page { view, _context: context, bounds, mode: request.mode, redirected })
    }

    /// Moves the page to `bounds` when it is elsewhere, and loads what was sent to the site directly through the proxy.
    pub fn place(&mut self, bounds: (i32, i32, u32, u32)) {
        if self.bounds != bounds {
            self.bounds = bounds;
            let _ = self.view.set_bounds(rect(bounds));
        }
        let pending = self.redirected.lock().map(|mut q| std::mem::take(&mut *q)).unwrap_or_default();
        if let Some(url) = pending.last() {
            let _ = self.view.load_url(url);
        }
    }

    pub fn navigate(&self, what: Navigate) {
        let home = proxy_url(self.mode.site(), self.mode.home()).unwrap_or_default();
        let _ = match what {
            Navigate::Home => self.view.load_url(&home),
            Navigate::Back => self.view.evaluate_script("history.back()"),
            Navigate::Forward => self.view.evaluate_script("history.forward()"),
        };
    }

    /// The page's address (the site's).
    pub fn url(&self) -> String {
        self.view.url().map(|u| site_url(self.mode.site(), &u)).unwrap_or_default()
    }
}

#[cfg(windows)]
fn rect((x, y, w, h): (i32, i32, u32, u32)) -> wry::Rect {
    wry::Rect { position: wry::dpi::PhysicalPosition::new(x, y).into(), size: wry::dpi::PhysicalSize::new(w.max(1), h.max(1)).into() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_headers_are_the_originals() {
        // (a ticket: id, address, port, the key's two cookies, then the event server type and the player id)
        let ticket = mao_net::system::Ticket { id: 7, address: [1, 2, 3, 4], port: 7501, key: [0, 0, 1, 0, 0, 0, 0, 2], rest: vec![0, 0, 0, 0, 0, 0, 0x30, 0x39] };
        let h = headers(1234, Some(&ticket), 3000, &|s| match s {
            59 => 1,
            4 => 2,
            47 => 3,
            64 => 4,
            _ => 0,
        });
        let get = |n: &str| h.iter().find(|(k, _)| k == n).map(|(_, v)| v.as_str());
        assert_eq!(get("X-Anarchy-CharacterID"), Some("1234"));
        assert_eq!(get("X-Anarchy-PlayerID"), Some("12345"));
        assert_eq!(get("X-Anarchy-DimensionType"), Some("0"));
        assert_eq!(get("X-Anarchy-Cookie"), Some("256 2"));
        assert_eq!(get("X-Anarchy-ServerID"), Some("3000"));
        assert_eq!(get("X-Anarchy-Appearance"), Some("1 2 3 4"));
        assert_eq!(get("X-Platform"), Some("Native"));
        assert!(page_script("en").contains("OpenMarketSendWindow"));
    }

    #[test]
    fn the_proxy_maps_the_sites_addresses() {
        let site = Mode::Market.site();
        assert_eq!(site_url(site, "http://aomarket.localhost/market/?q=1"), "http://aomarket.funcom.com/market/?q=1");
        assert_eq!(site_url(site, "aomarket://localhost/market/"), "http://aomarket.funcom.com/market/");
        assert_eq!(proxy_url(site, "http://aomarket.funcom.com/market/x").as_deref(), Some("http://aomarket.localhost/market/x"));
        assert_eq!(proxy_url(site, "http://example.com/"), None);
        assert_eq!(rewrite_text(site, "<a href=\"http://aomarket.funcom.com/a\">"), "<a href=\"http://aomarket.localhost/a\">");
        assert_eq!(rewrite_text(site, r#"{"u":"http:\/\/aomarket.funcom.com\/b"}"#), r#"{"u":"http:\/\/aomarket.localhost\/b"}"#);
        assert_eq!(rewrite_cookie("sid=1; Domain=.funcom.com; Path=/"), "sid=1; Path=/");
    }
}

/// `mao-client web store|market`: a window with nothing but the page (no game, so the headers are zeros), to see the page
/// and its proxy work.
#[cfg(windows)]
pub fn test_window(mode: Mode) -> Result<(), String> {
    use winit::application::ApplicationHandler;
    use winit::event::WindowEvent;
    use winit::event_loop::{ActiveEventLoop, EventLoop};
    use winit::window::{Window, WindowId};

    struct App {
        mode: Mode,
        window: Option<Window>,
        page: Option<Page>,
    }
    impl ApplicationHandler for App {
        fn resumed(&mut self, event_loop: &ActiveEventLoop) {
            if self.window.is_some() {
                return;
            }
            let window = event_loop.create_window(Window::default_attributes().with_title(self.mode.title()).with_inner_size(winit::dpi::PhysicalSize::new(1024, 768))).expect("a window");
            let size = window.inner_size();
            let request = Request { mode: self.mode, headers: headers(0, None, 0, &|_| 0) };
            let data = std::env::temp_dir().join("mao-browser-test");
            match Page::open(&window, &request, (0, 0, size.width, size.height), data, Default::default()) {
                Ok(page) => self.page = Some(page),
                Err(e) => {
                    eprintln!("the page could not be opened: {e}");
                    event_loop.exit();
                }
            }
            self.window = Some(window);
        }
        fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
            match event {
                WindowEvent::CloseRequested => event_loop.exit(),
                WindowEvent::Resized(size) => {
                    if let Some(page) = self.page.as_mut() {
                        page.place((0, 0, size.width, size.height));
                    }
                }
                _ => {}
            }
        }
        fn about_to_wait(&mut self, _: &ActiveEventLoop) {
            if let (Some(page), Some(window)) = (self.page.as_mut(), self.window.as_ref()) {
                let size = window.inner_size();
                page.place((0, 0, size.width, size.height));
            }
        }
    }
    let event_loop = EventLoop::new().map_err(|e| e.to_string())?;
    event_loop.set_control_flow(winit::event_loop::ControlFlow::wait_duration(std::time::Duration::from_millis(100)));
    event_loop.run_app(&mut App { mode, window: None, page: None }).map_err(|e| e.to_string())
}
