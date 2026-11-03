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

fn dashboard(db: &Db, message: &str) -> String {
    let mut rows = String::new();
    for (name, count) in db.list_accounts() {
        let chars: Vec<String> = db.character_names(&name).iter().map(|(id, n, level)| format!("{} (level {level}, id {id:#x})", escape(n))).collect();
        rows.push_str(&format!(
            "<tr><td>{}</td><td>{count}</td><td>{}</td><td><form method=post action=/accounts/delete onsubmit=\"return confirm('Delete this account and its characters?')\"><input type=hidden name=name value=\"{}\"><button>Delete</button></form></td></tr>",
            escape(&name),
            if chars.is_empty() { "-".to_string() } else { chars.join("<br>") },
            escape(&name)
        ));
    }
    if rows.is_empty() {
        rows = "<tr><td colspan=4>No accounts yet.</td></tr>".to_string();
    }
    let trace = escape(&crate::trace::tail(80));
    format!(
        "<!doctype html><html><head><meta charset=utf-8><title>AO server admin</title><style>{STYLE}</style></head><body>\
         <h2>AO server admin</h2><form method=post action=/logout style=\"float:right\"><button>Sign out</button></form>\
         <p class=note>{}</p>\
         <h3>Accounts</h3><table><tr><th>Account</th><th>Characters</th><th>Characters (level, id)</th><th></th></tr>{rows}</table>\
         <h3>New account</h3><form method=post action=/accounts>Name <input name=name maxlength=32 required> Password <input type=password name=password required> <button>Create</button></form>\
         <p class=note>Names: letters, digits, _ and -. The login accepts the account you make here.</p>\
         <h3>Trace (last 80 lines, <a href=/trace>full text</a>)</h3><pre>{trace}</pre>\
         </body></html>",
        escape(message)
    )
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
                        crate::trace::note("admin", &format!("account {name} made"));
                        format!("Account {name} is ready.")
                    }
                    Err(e) => format!("Could not save the account: {e}"),
                }
            };
            respond(&mut stream, "200 OK", &[], &dashboard(db, &message))
        }
        ("POST", "/accounts/delete") if logged_in => {
            let name = field("name");
            let message = match db.delete_account(&name) {
                Ok(()) => {
                    crate::trace::note("admin", &format!("account {name} removed"));
                    format!("Account {name} removed.")
                }
                Err(e) => format!("Could not remove {name}: {e}"),
            };
            respond(&mut stream, "200 OK", &[], &dashboard(db, &message))
        }
        _ => respond(&mut stream, if logged_in || request.path == "/" { "404 Not Found" } else { "303 See Other" }, &["Location: /".to_string()], "<p>Not here.</p>"),
    }
}
