//! A site's `robots.txt` (RFC 9309): which of its pages a program may fetch, and how slowly it asks to be fetched. The
//! crawler asks this before every page and takes a no for a no.

/// The rules that apply to one program.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Robots {
    /// (allowed, path pattern), as written.
    rules: Vec<(bool, String)>,
    /// `Crawl-delay`, in seconds, when the site asks for one.
    pub delay: Option<f32>,
    pub sitemaps: Vec<String>,
}

/// Reads a `robots.txt` for the program named `agent`: the rules of the group that names it (a group's name being part
/// of the program's, whatever the letters' case), else those of the group for everyone (`*`), else none.
pub fn parse(text: &str, agent: &str) -> Robots {
    let agent = agent.to_ascii_lowercase();
    // (the groups: the names a run of `User-agent` lines gives, and the lines under them)
    let mut groups: Vec<(Vec<String>, Vec<(String, String)>)> = Vec::new();
    let mut sitemaps = Vec::new();
    let mut naming = false;
    for line in text.lines() {
        let line = line.split('#').next().unwrap_or("").trim();
        let Some((key, value)) = line.split_once(':') else { continue };
        let (key, value) = (key.trim().to_ascii_lowercase(), value.trim().to_string());
        match key.as_str() {
            "user-agent" => {
                if !naming {
                    groups.push((Vec::new(), Vec::new()));
                }
                naming = true;
                if let Some(group) = groups.last_mut() {
                    group.0.push(value.to_ascii_lowercase());
                }
            }
            "sitemap" => sitemaps.push(value),
            _ => {
                naming = false;
                if let Some(group) = groups.last_mut() {
                    group.1.push((key, value));
                }
            }
        }
    }
    let named = groups.iter().filter(|g| g.0.iter().any(|n| n != "*" && !n.is_empty() && agent.contains(n.as_str()))).max_by_key(|g| g.0.iter().filter(|n| agent.contains(n.as_str())).map(String::len).max());
    let group = named.or_else(|| groups.iter().find(|g| g.0.iter().any(|n| n == "*")));
    let mut robots = Robots { sitemaps, ..Default::default() };
    for (key, value) in group.map(|g| g.1.as_slice()).unwrap_or_default() {
        match key.as_str() {
            "allow" if !value.is_empty() => robots.rules.push((true, value.clone())),
            "disallow" if !value.is_empty() => robots.rules.push((false, value.clone())),
            "crawl-delay" => robots.delay = value.parse().ok().filter(|d: &f32| d.is_finite() && *d >= 0.0),
            _ => {}
        }
    }
    robots
}

/// Whether a path fits a rule's pattern: the pattern's start, `*` for anything, `$` at its end for the path's end.
fn fits(pattern: &str, path: &str) -> bool {
    let (pattern, anchored) = match pattern.strip_suffix('$') {
        Some(p) => (p, true),
        None => (pattern, false),
    };
    let mut parts = pattern.split('*');
    let first = parts.next().unwrap_or("");
    if !path.starts_with(first) {
        return false;
    }
    let mut at = first.len();
    let rest: Vec<&str> = parts.collect();
    for (k, part) in rest.iter().enumerate() {
        if anchored && k + 1 == rest.len() {
            return path.len() >= at + part.len() && path[at..].ends_with(part);
        }
        match path[at..].find(part) {
            Some(found) => at += found + part.len(),
            None => return false,
        }
    }
    !anchored || at == path.len()
}

impl Robots {
    /// Whether the path (with its query) may be fetched: the longest rule that fits decides, an allow before a
    /// disallow of the same length; no rule that fits is a yes.
    pub fn allowed(&self, path: &str) -> bool {
        let path = if path.is_empty() { "/" } else { path };
        self.rules.iter().filter(|(_, pattern)| fits(pattern, path)).max_by_key(|(allow, pattern)| (pattern.len(), *allow)).is_none_or(|(allow, _)| *allow)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEXT: &str = "# a comment\nUser-agent: badbot\nUser-agent: otherbot\nDisallow: /\n\nUser-agent: *\nAllow: /\nDisallow: /admin/\nDisallow: /search\nDisallow: /*.rss$\nDisallow: /t/*/print\nAllow: /admin/public/\nCrawl-delay: 2.5\n\nUser-agent: helpbot\nDisallow: /private\n\nSitemap: https://site.example/sitemap.xml\n";

    #[test]
    fn everyones_rules_apply_to_a_program_not_named() {
        let robots = parse(TEXT, "SomeReader/1.0");
        assert_eq!(robots.delay, Some(2.5));
        assert_eq!(robots.sitemaps, ["https://site.example/sitemap.xml"]);
        assert!(robots.allowed("/guides/turnips"));
        assert!(robots.allowed("/"));
        assert!(!robots.allowed("/admin/users"));
        assert!(robots.allowed("/admin/public/page"));
        assert!(!robots.allowed("/search?q=x"));
        assert!(!robots.allowed("/c/news.rss"));
        assert!(robots.allowed("/c/news.rss.html"));
        assert!(!robots.allowed("/t/topic-name/print"));
        assert!(robots.allowed("/t/topic-name/12"));
        // (a private page is only closed to the program named for it)
        assert!(robots.allowed("/private/x"));
    }

    #[test]
    fn a_named_program_has_its_own_rules_only() {
        assert!(!parse(TEXT, "BadBot/2").allowed("/guides/turnips"));
        assert!(!parse(TEXT, "the-otherbot").allowed("/"));
        let own = parse(TEXT, "HelpBot");
        assert!(!own.allowed("/private/x"));
        assert!(own.allowed("/admin/users"));
        assert_eq!(own.delay, None);
    }

    #[test]
    fn no_rules_close_nothing() {
        assert!(parse("", "x").allowed("/anything"));
        assert!(parse("User-agent: *\nDisallow:\n", "x").allowed("/anything"));
        assert!(!parse("User-agent: *\nDisallow: /\n", "x").allowed("/anything"));
    }
}
