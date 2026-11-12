//! The help assistant's core. A player asks a question; the answer comes from pages kept and searched on the player's
//! own machine:
//!
//! - the game's own help files, read from the installed game each time (nothing of them is copied);
//! - the guides of sites the player has turned on, read slowly and by the site's rules into the player's settings
//!   folder ([`crawl`], [`index::Store`]).
//!
//! [`index`] ranks pieces of those pages for a question; [`llm`] can hand the best ones to a language model the player
//! has set up, for an answer in words with its sources named. Without a model the results themselves are the answer.
//! See `docs/HELP_ASSISTANT.md`.

pub mod crawl;
pub mod fetch;
pub mod index;
pub mod llm;
pub mod robots;
pub mod text;

use std::path::Path;

use index::{Doc, Index, Store};

/// The source that is the game's own help files.
pub const GAME: &str = "game";

/// The game's help pages (`cd_image/text/help/*.html`), each a page whose address is its file's name. The lists of
/// links (`index-*.html`) are left out: they say nothing themselves.
pub fn game_docs(game: &Path) -> Vec<Doc> {
    let dir = mao_formats::files::find(&game.join("cd_image/text/help"));
    let mut files: Vec<_> = std::fs::read_dir(&dir).map(|entries| entries.flatten().map(|e| e.path()).collect()).unwrap_or_default();
    files.sort();
    let mut docs = Vec::new();
    for file in files {
        let name = file.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
        if !name.to_ascii_lowercase().ends_with(".html") || name.to_ascii_lowercase().starts_with("index-") {
            continue;
        }
        let Ok(bytes) = std::fs::read(&file) else { continue };
        // (the files are Latin-1 where they are not plain ASCII)
        let raw: String = bytes.iter().map(|&b| b as char).collect();
        let mut page = text::extract(&raw, text::Reading { keep_newlines: true });
        if page.sections.is_empty() {
            continue;
        }
        if page.title.is_empty() {
            page.title = name.trim_end_matches(".html").replace('_', " ");
        }
        docs.push(Doc { source: GAME.to_string(), url: name, title: page.title, fetched: 0, stamp: String::new(), sections: page.sections });
    }
    docs
}

/// The index over the sources that are on: the game's help when `game` is given and `enabled` names it, and the kept
/// pages of the other sources `enabled` names.
pub fn load(store: &Store, game: Option<&Path>, enabled: &[String]) -> Index {
    let mut docs = Vec::new();
    for name in enabled {
        if name == GAME {
            if let Some(game) = game {
                docs.extend(game_docs(game));
            }
        } else {
            docs.extend(store.load(name));
        }
    }
    Index::build(docs)
}

/// The names in a `sources = game, ao-universe` setting, small letters, each once.
pub fn source_names(setting: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for name in setting.split(',').map(|n| n.trim().to_ascii_lowercase()).filter(|n| !n.is_empty()) {
        if !out.contains(&name) {
            out.push(name);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sources_setting_is_a_list_of_names() {
        assert_eq!(source_names(" Game, ao-universe ,,game"), ["game", "ao-universe"]);
        assert!(source_names("").is_empty());
    }

    /// `MAO_GAME`: the game's help files are found, read with their headlines, and answer a question about the game.
    #[test]
    fn the_games_help_answers_from_the_installed_game() {
        let Some(game) = std::env::var_os("MAO_GAME") else { return };
        let docs = game_docs(Path::new(&game));
        assert!(docs.len() > 80, "{} pages", docs.len());
        assert!(docs.iter().all(|d| !d.title.is_empty() && !d.url.starts_with("index-")));
        let index = Index::build(docs);
        let hits = index.search("what happens when my character dies, insurance", 5);
        assert!(hits.iter().any(|h| h.url.to_ascii_lowercase().contains("death")), "{:?}", hits.iter().map(|h| &h.url).collect::<Vec<_>>());
        // (no tag is left in what is shown)
        assert!(index.chunks.iter().all(|c| !c.text.contains("<font") && !c.text.contains("</a>")));
    }
}
