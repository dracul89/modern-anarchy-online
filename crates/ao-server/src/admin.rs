//! The admin panel: a web page to make and remove accounts, see the characters, and read the trace. It listens on `MAO_ADMIN_ADDR`
//! (default `127.0.0.1:8080`, this machine only) and needs `MAO_ADMIN_PASSWORD`: without that variable the panel does not start.
//! Accounts made here are the ones the login accepts. Passwords are kept as typed (this is a test server; see `docs/AO_SERVER.md`).

use std::collections::HashSet;
use std::hash::{BuildHasher, Hasher};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::{Arc, Mutex};

use crate::db::Db;

const COOKIE: &str = "ao_admin";

/// Starts the panel (does nothing without `MAO_ADMIN_PASSWORD`).
pub fn spawn(addr: SocketAddr, db: Arc<Db>) -> std::io::Result<()> {
    let Ok(password) = std::env::var("MAO_ADMIN_PASSWORD") else {
        eprintln!("ao-server: the admin panel is off (set MAO_ADMIN_PASSWORD to turn it on)");
        return Ok(());
    };
    let listener = TcpListener::bind(addr)?;
    let sessions: Arc<Mutex<HashSet<String>>> = Arc::default();
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let (db, sessions, password) = (db.clone(), sessions.clone(), password.clone());
            std::thread::spawn(move || {
                let _ = handle(stream, &db, &sessions, &password);
            });
        }
    });
    println!("ao-server: admin panel on http://{addr}");
    Ok(())
}

struct Request {
    method: String,
    path: String,
    cookie: Option<String>,
    form: Vec<(String, String)>,
}

/// The percent-decoded value of a form field (`+` is a space).
fn decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => out.push(b' '),
            b'%' if i + 2 < bytes.len() => {
                if let Ok(v) = u8::from_str_radix(&text[i + 1..i + 3], 16) {
                    out.push(v);
                    i += 3;
                    continue;
                }
                out.push(b'%');
            }
            b => out.push(b),
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn parse_form(text: &str) -> Vec<(String, String)> {
    text.split('&').filter(|p| !p.is_empty()).map(|p| {
        let (k, v) = p.split_once('=').unwrap_or((p, ""));
        (decode(k), decode(v))
    }).collect()
}

fn read_request(stream: &mut TcpStream) -> Option<Request> {
    let mut reader = BufReader::new(stream.try_clone().ok()?);
    let mut line = String::new();
    reader.read_line(&mut line).ok()?;
    let mut parts = line.split_whitespace();
    let method = parts.next()?.to_string();
    let path = parts.next()?.split('?').next()?.to_string();
    let (mut length, mut cookie) = (0usize, None);
    loop {
        let mut header = String::new();
        if reader.read_line(&mut header).ok()? == 0 || header.trim().is_empty() {
            break;
        }
        if let Some((name, value)) = header.split_once(':') {
            match name.trim().to_ascii_lowercase().as_str() {
                "content-length" => length = value.trim().parse().unwrap_or(0),
                "cookie" => cookie = value.split(';').find_map(|c| c.trim().strip_prefix(&format!("{COOKIE}=")).map(str::to_string)),
                _ => {}
            }
        }
    }
    let mut body = vec![0u8; length.min(65536)];
    reader.read_exact(&mut body).ok()?;
    let form = parse_form(&String::from_utf8_lossy(&body));
    Some(Request { method, path, cookie, form })
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn respond(stream: &mut TcpStream, status: &str, headers: &[String], body: &str) -> std::io::Result<()> {
    let mut head = format!("HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n", body.len());
    for h in headers {
        head.push_str(h);
        head.push_str("\r\n");
    }
    head.push_str("\r\n");
    stream.write_all(head.as_bytes())?;
    stream.write_all(body.as_bytes())
}

fn redirect(stream: &mut TcpStream, to: &str, cookie: Option<String>) -> std::io::Result<()> {
    let mut headers = vec![format!("Location: {to}")];
    if let Some(c) = cookie {
        headers.push(c);
    }
    respond(stream, "303 See Other", &headers, "")
}

fn new_token() -> String {
    let mut out = String::new();
    for _ in 0..2 {
        let mut h = std::collections::hash_map::RandomState::new().build_hasher();
        h.write_u128(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0));
        out.push_str(&format!("{:016x}", h.finish()));
    }
    out
}

const STYLE: &str = "body{font-family:sans-serif;margin:24px;max-width:960px}table{border-collapse:collapse;margin:8px 0}td,th{border:1px solid #bbb;padding:4px 8px;text-align:left}pre{background:#111;color:#ddd;padding:8px;overflow:auto;max-height:420px;font-size:12px}input{padding:4px}.note{color:#555;font-size:13px}";

fn login_page(message: &str) -> String {
    format!("<!doctype html><html><head><meta charset=utf-8><title>AO server admin</title><style>{STYLE}</style></head><body><h2>AO server admin</h2><p class=note>{}</p><form method=post action=/login><input type=password name=password placeholder=\"admin password\" autofocus> <button>Sign in</button></form></body></html>", escape(message))
}

/// A number typed in a form: decimal, or hexadecimal with `0x`.
fn number(text: &str) -> Option<i64> {
    match text.strip_prefix("0x") {
        Some(hex) => i64::from_str_radix(hex, 16).ok(),
        None => text.parse().ok(),
    }
}

/// One character's row: what it is and what it has, with the forms that change it.
fn character_row(db: &Db, account: &str, id: u32, name: &str) -> String {
    let (level, xp, credits, items, missions, playfield) = match db.load_hero(id) {
        Some(h) => (h.level, h.xp, h.credits, h.items.len(), h.missions.len(), h.playfield),
        None => (1, 0, 0, 0, 0, 0),
    };
    format!(
        "<tr><td>{name}<br><span class=note>id {id} ({id:#x})</span></td><td>{level}</td><td>{xp}</td><td>{credits}</td>\
         <td>{items} items, {missions} missions<br><span class=note>playfield {playfield}</span></td>\
         <td><form method=post action=/characters/xp><input type=hidden name=id value={id}><input name=amount size=8 placeholder=xp> <button>Give XP</button></form>\
         <form method=post action=/characters/credits><input type=hidden name=id value={id}><input name=amount size=8 placeholder=credits> <button>Give credits</button></form>\
         <form method=post action=/characters/item><input type=hidden name=id value={id}><input name=template size=10 placeholder=template> <input name=count size=3 value=1> <button>Give item</button></form>\
         <form method=post action=/characters/delete onsubmit=\"return confirm('Delete this character?')\"><input type=hidden name=id value={id}><input type=hidden name=account value=\"{account}\"><button>Delete character</button></form></td></tr>",
        name = escape(name),
        account = escape(account),
    )
}

fn dashboard(db: &Db, message: &str) -> String {
    let mut blocks = String::new();
    for (name, _) in db.list_accounts() {
        let (slots, expansions) = db.account_settings(&name).unwrap_or((6, 1));
        let mut rows = String::new();
        for c in db.characters_of(&name) {
            rows.push_str(&character_row(db, &name, c.id, &c.name));
        }
        if rows.is_empty() {
            rows = "<tr><td colspan=6 class=note>No characters yet.</td></tr>".to_string();
        }
        blocks.push_str(&format!(
            "<h3>{name}</h3>\
             <form method=post action=/accounts/settings>Character slots <input name=slots size=3 value={slots}> Expansion flags <input name=expansions size=6 value={expansions}> <input type=hidden name=name value=\"{name}\"><button>Save</button></form>\
             <form method=post action=/accounts/delete onsubmit=\"return confirm('Delete this account and its characters?')\"><input type=hidden name=name value=\"{name}\"><button>Delete account</button></form>\
             <table><tr><th>Character</th><th>Level</th><th>XP</th><th>Credits</th><th>Carries</th><th>Actions</th></tr>{rows}</table>",
            name = escape(&name)
        ));
    }
    if blocks.is_empty() {
        blocks = "<p>No accounts yet.</p>".to_string();
    }
    let trace = escape(&crate::trace::tail(80));
    format!(
        "<!doctype html><html><head><meta charset=utf-8><title>AO server admin</title><style>{STYLE}</style></head><body>\
         <h2>AO server admin</h2><form method=post action=/logout style=\"float:right\"><button>Sign out</button></form>\
         <p class=note>{}</p>\
         {blocks}\
         <h3>New account</h3><form method=post action=/accounts>Name <input name=name maxlength=32 required> Password <input type=password name=password required> <button>Create</button></form>\
         <p class=note>Names: letters, digits, _ and -. The login accepts the account you make here. Changes to a character take effect at its next login.</p>\
         <h3>Trace (last 80 lines, <a href=/trace>full text</a>)</h3><pre>{trace}</pre>\
         </body></html>",
        escape(message)
    )
}

/// Gives a character experience, credits or an item (the panel's forms). Returns the message to show.
fn give(db: &Db, path: &str, id: &str, amount: &str, count: &str, template: &str) -> String {
    let Some(id) = number(id).map(|n| n as u32) else { return "No character.".to_string() };
    let Some(mut hero) = db.load_hero(id) else { return format!("There is no character {id}.") };
    let what = match path {
        "/characters/xp" => match number(amount).filter(|a| *a > 0) {
            Some(a) => {
                let up = hero.grant_xp(a as u64);
                format!("{a} XP{}", if up { format!(" (now level {})", hero.level) } else { String::new() })
            }
            None => return "The amount must be a number above zero.".to_string(),
        },
        "/characters/credits" => match number(amount).filter(|a| *a > 0) {
            Some(a) => {
                hero.credits = hero.credits.saturating_add(a as u32);
                format!("{a} credits")
            }
            None => return "The amount must be a number above zero.".to_string(),
        },
        _ => {
            let count = number(count).unwrap_or(1).clamp(1, 1000) as u16;
            match number(template).map(|t| t as u32).and_then(|t| hero.add_item(t, count, 1).map(|_| t)) {
                Some(t) => format!("item {t:#x} x{count}"),
                None => return "The item needs a template number, and the backpack must have room.".to_string(),
            }
        }
    };
    match db.save_hero(&hero) {
        Ok(()) => {
            crate::trace::note(&format!("character {id}: given {what}"));
            format!("Character {id} was given {what}.")
        }
        Err(e) => format!("Could not save: {e}"),
    }
}

fn handle(mut stream: TcpStream, db: &Db, sessions: &Mutex<HashSet<String>>, password: &str) -> std::io::Result<()> {
    let Some(request) = read_request(&mut stream) else { return Ok(()) };
    let logged_in = request.cookie.as_ref().is_some_and(|t| sessions.lock().unwrap().contains(t));
    let field = |name: &str| request.form.iter().find(|(k, _)| k == name).map(|(_, v)| v.trim().to_string()).unwrap_or_default();
    match (request.method.as_str(), request.path.as_str()) {
        ("GET", "/") => {
            let page = if logged_in { dashboard(db, "") } else { login_page("") };
            respond(&mut stream, "200 OK", &[], &page)
        }
        ("GET", "/trace") if logged_in => {
            let text = crate::trace::tail(100_000);
            let head = "HTTP/1.1 200 OK\r\nContent-Type: text/plain; charset=utf-8\r\nConnection: close\r\n";
            stream.write_all(format!("{head}Content-Length: {}\r\n\r\n", text.len()).as_bytes())?;
            stream.write_all(text.as_bytes())
        }
        ("POST", "/login") => {
            if field("password") == password {
                let token = new_token();
                sessions.lock().unwrap().insert(token.clone());
                redirect(&mut stream, "/", Some(format!("Set-Cookie: {COOKIE}={token}; Path=/; HttpOnly")))
            } else {
                respond(&mut stream, "200 OK", &[], &login_page("That password is not the admin password."))
            }
        }
        ("POST", "/logout") => {
            if let Some(t) = &request.cookie {
                sessions.lock().unwrap().remove(t);
            }
            redirect(&mut stream, "/", Some(format!("Set-Cookie: {COOKIE}=; Path=/; Max-Age=0")))
        }
        ("POST", "/accounts") if logged_in => {
            let (name, pass) = (field("name"), field("password"));
            let valid = !name.is_empty() && name.len() <= 32 && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
            let message = if !valid {
                "The name needs letters, digits, _ or - (up to 32).".to_string()
            } else if pass.is_empty() {
                "The account needs a password.".to_string()
            } else {
                match db.save_account(&name, &pass) {
                    Ok(()) => {
                        crate::trace::note(&format!("account {name} made"));
                        format!("Account {name} is ready.")
                    }
                    Err(e) => format!("Could not save the account: {e}"),
                }
            };
            respond(&mut stream, "200 OK", &[], &dashboard(db, &message))
        }
        ("POST", "/accounts/settings") if logged_in => {
            let name = field("name");
            let slots = number(&field("slots")).filter(|n| (1..=100).contains(n));
            let expansions = number(&field("expansions")).filter(|n| (0..=u32::MAX as i64).contains(n));
            let message = match (slots, expansions) {
                (Some(slots), Some(expansions)) if db.account_exists(&name) => match db.set_account_settings(&name, slots, expansions) {
                    Ok(()) => {
                        crate::trace::note(&format!("account {name}: {slots} slots, expansion flags {expansions:#x}"));
                        format!("{name} now has {slots} character slots.")
                    }
                    Err(e) => format!("Could not save: {e}"),
                },
                (None, _) => "The slots are a number from 1 to 100.".to_string(),
                (_, None) => "The expansion flags are a number.".to_string(),
                _ => format!("There is no account {name}."),
            };
            respond(&mut stream, "200 OK", &[], &dashboard(db, &message))
        }
        ("POST", "/characters/xp") | ("POST", "/characters/credits") | ("POST", "/characters/item") if logged_in => {
            let message = give(db, &request.path, &field("id"), &field("amount"), &field("count"), &field("template"));
            respond(&mut stream, "200 OK", &[], &dashboard(db, &message))
        }
        ("POST", "/characters/delete") if logged_in => {
            let (id, account) = (number(&field("id")), field("account"));
            let message = match id {
                Some(id) => match db.delete_character(&account, id) {
                    Ok(()) => {
                        crate::trace::note(&format!("character {id} of {account} deleted"));
                        format!("Character {id} deleted.")
                    }
                    Err(e) => format!("Could not delete: {e}"),
                },
                None => "No character.".to_string(),
            };
            respond(&mut stream, "200 OK", &[], &dashboard(db, &message))
        }
        ("POST", "/accounts/delete") if logged_in => {
            let name = field("name");
            let message = match db.delete_account(&name) {
                Ok(()) => {
                    crate::trace::note(&format!("account {name} removed"));
                    format!("Account {name} removed.")
                }
                Err(e) => format!("Could not remove {name}: {e}"),
            };
            respond(&mut stream, "200 OK", &[], &dashboard(db, &message))
        }
        _ => respond(&mut stream, if logged_in || request.path == "/" { "404 Not Found" } else { "303 See Other" }, &["Location: /".to_string()], "<p>Not here.</p>"),
    }
}
