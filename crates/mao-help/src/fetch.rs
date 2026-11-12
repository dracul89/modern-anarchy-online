//! Fetching: the one place the help assistant talks to the network. [`Http`] is what the crawler and the answer step
//! ask of it, so that tests give them made-up pages; [`Curl`] is the real one, the system's `curl` program run for each
//! request (Windows 10 and later, macOS and nearly every Linux have it), which brings TLS without a TLS library in the
//! client. Headers and the request's body never go on the command line, where other programs could read them: curl
//! reads them from its standard input (`--config -`) and from a file in the help folder.
//!
//! Every request names the program plainly ([`AGENT`]) and carries nothing of the player: no account, no character,
//! no cookies.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// How the program names itself to the sites it reads (and the name it looks for in their `robots.txt`).
pub const AGENT: &str = "ModernAnarchyOnline-help/0.1 (a player's own help index)";

/// An answer: the status and the body.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Reply {
    pub status: u16,
    pub body: Vec<u8>,
}

impl Reply {
    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }

    pub fn ok(&self) -> bool {
        (200..300).contains(&self.status)
    }
}

/// What the crawler and the answer step need of the network.
pub trait Http {
    fn get(&mut self, url: &str) -> Result<Reply, String>;
    /// Sends JSON with the given headers and returns the answer.
    fn post_json(&mut self, url: &str, headers: &[(String, String)], body: &str) -> Result<Reply, String>;
    /// Waits until `gap` has passed since the last request to a site (the real one sleeps; a test's does not).
    fn wait(&mut self, gap: Duration);
}

/// The system's `curl`.
pub struct Curl {
    /// Where a request's body is put for curl to read.
    pub scratch: PathBuf,
    pub timeout: Duration,
    last: Option<Instant>,
}

impl Curl {
    pub fn new(scratch: impl Into<PathBuf>) -> Curl {
        Curl { scratch: scratch.into(), timeout: Duration::from_secs(30), last: None }
    }

    /// Whether the program is there to be run.
    pub fn available() -> bool {
        Command::new("curl").arg("--version").stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).status().is_ok_and(|s| s.success())
    }

    /// Runs curl with `config` (its option file's lines) on its standard input; the status is the answer's last line
    /// (`--write-out`), cut off again here.
    fn run(&mut self, config: String) -> Result<Reply, String> {
        self.last = Some(Instant::now());
        let mut command = Command::new("curl");
        // (no console window of curl's own beside a client that has none)
        #[cfg(windows)]
        std::os::windows::process::CommandExt::creation_flags(&mut command, 0x0800_0000);
        let mut child = command
            .args(["--silent", "--show-error", "--location", "--max-redirs", "5", "--config", "-", "--write-out", "\n%{http_code}"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("curl could not be started: {e}"))?;
        if let Some(mut input) = child.stdin.take() {
            input.write_all(config.as_bytes()).map_err(|e| e.to_string())?;
        }
        let out = child.wait_with_output().map_err(|e| e.to_string())?;
        if !out.status.success() {
            return Err(format!("curl: {}", String::from_utf8_lossy(&out.stderr).trim()));
        }
        let cut = out.stdout.iter().rposition(|&b| b == b'\n').ok_or("curl gave no status")?;
        let status = String::from_utf8_lossy(&out.stdout[cut + 1..]).trim().parse().map_err(|_| "curl gave no status")?;
        Ok(Reply { status, body: out.stdout[..cut].to_vec() })
    }

    fn base(&self, url: &str) -> String {
        format!("url = {}\nuser-agent = {}\nmax-time = {}\n", quoted(url), quoted(AGENT), self.timeout.as_secs().max(1))
    }
}

/// A value for curl's option file: in quotes, with `\` before a quote or a backslash; a line end cannot be written.
pub fn quoted(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for c in value.chars() {
        match c {
            '"' | '\\' => {
                out.push('\\');
                out.push(c);
            }
            '\n' | '\r' => out.push(' '),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

impl Http for Curl {
    fn get(&mut self, url: &str) -> Result<Reply, String> {
        let config = self.base(url);
        self.run(config)
    }

    fn post_json(&mut self, url: &str, headers: &[(String, String)], body: &str) -> Result<Reply, String> {
        std::fs::create_dir_all(&self.scratch).map_err(|e| e.to_string())?;
        let file = self.scratch.join(format!("request-{}.json", std::process::id()));
        std::fs::write(&file, body).map_err(|e| e.to_string())?;
        let mut config = self.base(url);
        config += "header = \"Content-Type: application/json\"\n";
        for (name, value) in headers {
            config += &format!("header = {}\n", quoted(&format!("{name}: {value}")));
        }
        config += &format!("data-binary = {}\n", quoted(&format!("@{}", file.to_string_lossy().replace('\\', "/"))));
        // (a model thinking on a small machine takes its time)
        config = config.replace(&format!("max-time = {}\n", self.timeout.as_secs().max(1)), "max-time = 300\n");
        let reply = self.run(config);
        let _ = std::fs::remove_file(&file);
        reply
    }

    fn wait(&mut self, gap: Duration) {
        if let Some(left) = self.last.and_then(|at| gap.checked_sub(at.elapsed())) {
            std::thread::sleep(left);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn option_values_are_quoted_for_curl() {
        assert_eq!(quoted("https://site.example/a?b=c"), "\"https://site.example/a?b=c\"");
        assert_eq!(quoted("Authorization: Bearer a\"b\\c\nd"), "\"Authorization: Bearer a\\\"b\\\\c d\"");
    }
}
