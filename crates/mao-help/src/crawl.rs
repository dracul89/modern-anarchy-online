//! Reading a site into the player's own help folder: its `robots.txt` first, then the pages its sitemap lists under
//! the paths that hold guides, one at a time and slowly. What is fetched stays on the player's machine
//! ([`crate::index::Store`]); nothing of it is in the repository or in a package.
//!
//! The sites known here ([`SITES`]) are off until the player turns one on, and each says what its owner's terms are
//! ([`Site::terms`]): see `docs/HELP_ASSISTANT.md` for how they were read.

use std::time::Duration;

use crate::fetch::{Http, AGENT};
use crate::index::Doc;
use crate::robots;
use crate::text::{extract, Reading};

/// A site whose guides can be read by its sitemap.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Site {
    /// The source's name in the settings and in the help folder.
    pub name: &'static str,
    pub label: &'static str,
    /// `https://host`, without a slash at the end.
    pub origin: &'static str,
    /// The sitemap's path.
    pub sitemap: &'static str,
    /// The paths whose pages are read (their beginnings).
    pub include: &'static [&'static str],
    /// The least time between two requests, in seconds (a site's `Crawl-delay` may make it longer).
    pub gap: f32,
    /// What the site says about its content, for the player to read before turning it on.
    pub terms: &'static str,
}

/// The sites the assistant knows how to read. None is read unless the player's settings name it.
pub const SITES: &[Site] = &[Site {
    name: "ao-universe",
    label: "AO-Universe guides",
    origin: "https://www.ao-universe.com",
    sitemap: "/sitemap.xml",
    include: &["/guides/"],
    gap: 2.0,
    terms: "AO-Universe's pages say: \"Website Content may not be reproduced without permission\". Its robots.txt lets programs read the guides. The copy made here stays on this machine for your own searching and is never passed on; results link to the site.",
}];

pub fn site(name: &str) -> Option<&'static Site> {
    SITES.iter().find(|s| s.name.eq_ignore_ascii_case(name.trim()))
}

/// The pages a sitemap lists: (address, the site's date for it). A sitemap of sitemaps gives its sitemaps' addresses
/// the same way.
pub fn sitemap_entries(xml: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut rest = xml;
    while let Some(start) = rest.find("<loc>") {
        rest = &rest[start + 5..];
        let Some(end) = rest.find("</loc>") else { break };
        let address = crate::text::decode_entities(rest[..end].trim());
        rest = &rest[end + 6..];
        // (the date belongs to this entry when it comes before the next address)
        let next = rest.find("<loc>").unwrap_or(rest.len());
        let stamp = rest[..next].find("<lastmod>").and_then(|s| rest[s + 9..next].find("</lastmod>").map(|e| rest[s + 9..s + 9 + e].trim().to_string())).unwrap_or_default();
        out.push((address, stamp));
    }
    out
}

/// An address's path and query on `origin`, or nothing when it is another site's.
pub fn path_on<'a>(origin: &str, address: &'a str) -> Option<&'a str> {
    let rest = address.strip_prefix(origin)?;
    if rest.is_empty() {
        Some("/")
    } else if rest.starts_with('/') {
        Some(rest)
    } else {
        None
    }
}

/// How a crawl went.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Report {
    /// The pages the sitemap lists under the site's guide paths.
    pub listed: usize,
    pub fetched: usize,
    /// Pages kept from before, their date unchanged.
    pub kept: usize,
    /// Pages the site's robots.txt closes.
    pub closed: usize,
    pub failed: usize,
    /// More pages were due than this run's limit.
    pub left: usize,
}

/// What a crawl tells while it runs.
pub enum Step<'a> {
    /// A page is about to be fetched: its address, which one it is and of how many.
    Fetching(&'a str, usize, usize),
    /// Every [`CHECKPOINT`] pages: all the pages to keep as things stand, for the caller to write down, so that a
    /// reading that is cut short (the game closed) loses little.
    Checkpoint(&'a [Doc]),
}

/// How many fetched pages lie between two checkpoints.
pub const CHECKPOINT: usize = 20;

/// Reads a site: `known` are the pages kept from before (a page whose date in the sitemap is the one it was fetched
/// with is not fetched again), `limit` the most pages fetched in this run. Returns the pages to keep and what happened.
/// `now` is the time written into the fetched pages; `stop` ends the run early (the pages so far are returned).
pub fn crawl(http: &mut dyn Http, site: &Site, known: Vec<Doc>, limit: usize, now: u64, progress: &mut dyn FnMut(Step<'_>), stop: &dyn Fn() -> bool) -> Result<(Vec<Doc>, Report), String> {
    // (the site's rules: no file at all closes nothing; a file that cannot be had closes everything)
    let robots_reply = http.get(&format!("{}/robots.txt", site.origin))?;
    let rules = match robots_reply.status {
        200..=299 => robots::parse(&robots_reply.text(), AGENT),
        404 | 410 => robots::Robots::default(),
        status => return Err(format!("{}'s robots.txt could not be read (status {status}): nothing is fetched without it", site.label)),
    };
    let gap = Duration::from_secs_f32(site.gap.max(rules.delay.unwrap_or(0.0)).clamp(1.0, 60.0));
    if !rules.allowed(site.sitemap) {
        return Err(format!("{}'s robots.txt closes its sitemap", site.label));
    }
    http.wait(gap);
    let map = http.get(&format!("{}{}", site.origin, site.sitemap))?;
    if !map.ok() {
        return Err(format!("{}'s sitemap could not be read (status {})", site.label, map.status));
    }
    let mut report = Report::default();
    let listed: Vec<(String, String)> = sitemap_entries(&map.text()).into_iter().filter(|(address, _)| path_on(site.origin, address).is_some_and(|p| site.include.iter().any(|i| p.starts_with(i)))).collect();
    report.listed = listed.len();
    let mut known: std::collections::HashMap<String, Doc> = known.into_iter().map(|d| (d.url.clone(), d)).collect();
    let mut docs = Vec::new();
    let due = listed.iter().filter(|(address, stamp)| known.get(address).is_none_or(|d| d.stamp != *stamp)).count();
    for (address, stamp) in &listed {
        let path = path_on(site.origin, address).unwrap_or("/");
        if !rules.allowed(path) {
            report.closed += 1;
            continue;
        }
        if let Some(old) = known.remove(address) {
            if old.stamp == *stamp || report.fetched >= limit || stop() {
                // (unchanged, or not this run's turn: the old copy stays)
                report.kept += usize::from(old.stamp == *stamp);
                report.left += usize::from(old.stamp != *stamp);
                docs.push(old);
                continue;
            }
        } else if report.fetched >= limit || stop() {
            report.left += 1;
            continue;
        }
        progress(Step::Fetching(address, report.fetched + 1, due.min(limit)));
        http.wait(gap);
        match http.get(address) {
            Ok(reply) if reply.ok() => {
                let page = extract(&reply.text(), Reading::default());
                report.fetched += 1;
                if !page.sections.is_empty() {
                    docs.push(Doc { source: site.name.to_string(), url: address.clone(), title: page.title, fetched: now, stamp: stamp.clone(), sections: page.sections });
                }
                if report.fetched % CHECKPOINT == 0 {
                    // (what was read so far, and the old copies of the pages not yet come to)
                    let mut all = docs.clone();
                    all.extend(known.values().cloned());
                    progress(Step::Checkpoint(&all));
                }
            }
            _ => {
                report.fetched += 1;
                report.failed += 1;
            }
        }
    }
    Ok((docs, report))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::fetch::Reply;

    /// A made-up web: addresses and what they answer; every request and every wait is written down.
    #[derive(Default)]
    pub struct Fake {
        pub pages: std::collections::HashMap<String, (u16, String)>,
        pub asked: Vec<String>,
        pub posted: Vec<(String, Vec<(String, String)>, String)>,
        pub waits: Vec<Duration>,
    }

    impl Fake {
        pub fn page(mut self, url: &str, status: u16, body: &str) -> Fake {
            self.pages.insert(url.to_string(), (status, body.to_string()));
            self
        }
    }

    impl Http for Fake {
        fn get(&mut self, url: &str) -> Result<Reply, String> {
            self.asked.push(url.to_string());
            let (status, body) = self.pages.get(url).cloned().unwrap_or((404, String::new()));
            Ok(Reply { status, body: body.into_bytes() })
        }

        fn post_json(&mut self, url: &str, headers: &[(String, String)], body: &str) -> Result<Reply, String> {
            self.posted.push((url.to_string(), headers.to_vec(), body.to_string()));
            let (status, body) = self.pages.get(url).cloned().unwrap_or((404, String::new()));
            Ok(Reply { status, body: body.into_bytes() })
        }

        fn wait(&mut self, gap: Duration) {
            self.waits.push(gap);
        }
    }

    const SITE: Site = Site { name: "garden", label: "The Garden", origin: "https://garden.example", sitemap: "/sitemap.xml", include: &["/guides/"], gap: 2.0, terms: "" };

    fn web() -> Fake {
        Fake::default()
            .page("https://garden.example/robots.txt", 200, "User-agent: *\nDisallow: /guides/secret/\nCrawl-delay: 3\n")
            .page(
                "https://garden.example/sitemap.xml",
                200,
                "<urlset><url><loc>https://garden.example/news</loc><lastmod>d0</lastmod></url><url>\n<loc>https://garden.example/guides/turnips</loc>\n<lastmod>d1</lastmod></url><url><loc>https://garden.example/guides/secret/plan</loc></url><url><loc>https://garden.example/guides/beans?a=1&amp;b=2</loc><lastmod>d2</lastmod></url><url><loc>https://elsewhere.example/guides/x</loc></url><url><loc>https://garden.example/guides/gone</loc></url></urlset>",
            )
            .page("https://garden.example/guides/turnips", 200, "<html><head><title>Turnips | The Garden</title></head><body><article><h1>Turnips</h1><p>Cold loam.</p></article></body></html>")
            .page("https://garden.example/guides/beans?a=1&b=2", 200, "<html><body><article><h1>Beans</h1><p>Tall poles.</p></article></body></html>")
    }

    #[test]
    fn a_sitemap_gives_addresses_and_dates() {
        assert_eq!(sitemap_entries("<urlset><url><loc> a </loc><lastmod>d1</lastmod></url><url><loc>b&amp;c</loc></url><url><loc>d</loc><lastmod>d3</lastmod></url></urlset>"), [("a".to_string(), "d1".to_string()), ("b&c".to_string(), String::new()), ("d".to_string(), "d3".to_string())]);
        assert_eq!(path_on("https://a.example", "https://a.example/x?y"), Some("/x?y"));
        assert_eq!(path_on("https://a.example", "https://a.example"), Some("/"));
        assert_eq!(path_on("https://a.example", "https://a.example.evil/x"), None);
    }

    #[test]
    fn a_site_is_read_by_its_rules() {
        let mut http = web();
        let (docs, report) = crawl(&mut http, &SITE, Vec::new(), 10, 77, &mut |_| {}, &|| false).unwrap();
        // (the guides only; not the closed one, not another site's; the missing one counted as failed)
        assert_eq!(http.asked, ["https://garden.example/robots.txt", "https://garden.example/sitemap.xml", "https://garden.example/guides/turnips", "https://garden.example/guides/beans?a=1&b=2", "https://garden.example/guides/gone"]);
        assert_eq!(report, Report { listed: 4, fetched: 3, kept: 0, closed: 1, failed: 1, left: 0 });
        assert_eq!(docs.iter().map(|d| (d.url.as_str(), d.title.as_str(), d.stamp.as_str(), d.fetched)).collect::<Vec<_>>(), [("https://garden.example/guides/turnips", "Turnips", "d1", 77), ("https://garden.example/guides/beans?a=1&b=2", "Beans", "d2", 77)]);
        assert_eq!(docs[0].sections[0].text, "Cold loam.");
        // (the site's own delay, longer than ours, before every request after the first)
        assert_eq!(http.waits, vec![Duration::from_secs(3); 4]);

        // (again: what has not changed is not fetched again; a changed date is)
        let mut http = web().page("https://garden.example/guides/turnips", 200, "<article><h1>Turnips</h1><p>Warm sand.</p></article>");
        let mut known = docs.clone();
        known[1].stamp = "old".into();
        let (again, report) = crawl(&mut http, &SITE, known, 10, 99, &mut |_| {}, &|| false).unwrap();
        assert_eq!(report, Report { listed: 4, fetched: 2, kept: 1, closed: 1, failed: 1, left: 0 });
        assert_eq!(again[0], docs[0]);
        assert_eq!((again[1].fetched, again[1].stamp.as_str()), (99, "d2"));
    }

    #[test]
    fn a_run_has_a_limit_and_can_be_stopped() {
        let mut http = web();
        let (docs, report) = crawl(&mut http, &SITE, Vec::new(), 1, 1, &mut |_| {}, &|| false).unwrap();
        assert_eq!((docs.len(), report.fetched, report.left), (1, 1, 2));
        let mut http = web();
        let (docs, report) = crawl(&mut http, &SITE, Vec::new(), 10, 1, &mut |_| {}, &|| true).unwrap();
        assert_eq!((docs.len(), report.fetched, report.left), (0, 0, 3));
    }

    #[test]
    fn nothing_is_fetched_against_the_sites_rules() {
        // (robots.txt closes everything: the sitemap is not asked for)
        let mut http = web().page("https://garden.example/robots.txt", 200, "User-agent: *\nDisallow: /\n");
        assert!(crawl(&mut http, &SITE, Vec::new(), 10, 1, &mut |_| {}, &|| false).is_err());
        assert_eq!(http.asked, ["https://garden.example/robots.txt"]);
        // (the program named and closed out)
        let mut http = web().page("https://garden.example/robots.txt", 200, "User-agent: ModernAnarchyOnline-help\nDisallow: /\n\nUser-agent: *\nAllow: /\n");
        assert!(crawl(&mut http, &SITE, Vec::new(), 10, 1, &mut |_| {}, &|| false).is_err());
        // (robots.txt that cannot be had: a server error is not a yes)
        let mut http = web().page("https://garden.example/robots.txt", 503, "");
        assert!(crawl(&mut http, &SITE, Vec::new(), 10, 1, &mut |_| {}, &|| false).is_err());
        assert_eq!(http.asked.len(), 1);
        // (no robots.txt at all closes nothing)
        let mut http = web().page("https://garden.example/robots.txt", 404, "");
        assert_eq!(crawl(&mut http, &SITE, Vec::new(), 10, 1, &mut |_| {}, &|| false).unwrap().1.fetched, 4);
    }
}
