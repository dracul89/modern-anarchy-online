# The help assistant ("Ask the guides")

A window in the client where a player types a question about the game and gets the pieces of help pages and
player-written guides that fit it best, each with where it is from; and, when the player has set up a language model,
an answer in words written from those pieces. This is the client's own: the original has nothing like it.

Started 2026-11-12 on the user's note: "Start looking at scraping ao-universe and other related sites for
integrated via a small LLM help window". This document is the look at the sites, the design, and what the first cut
does. Branch `agent/help-assistant`.

The short of it:

- **Nothing scraped is in the repository or in a package, and nothing is meant to be.** The player's own client reads
  the pages, on the player's machine, into the player's settings folder, for the player's own searching. The tests
  use made-up pages only.
- **The game's own help files are the first source** and need no network: 110 pages in the installed game's
  `cd_image/text/help`, read from there each time.
- **One site is wired up, AO-Universe's guides, and it is off until the player turns it on.** Its robots.txt allows
  it; its footer says its content may not be reproduced without permission, which a private copy for searching does
  not do, but which is a reason to ask its team before this is ever on by default (see "Open questions").
- **The other sites looked at are not wired up**, each for a reason given below.
- **No model is needed.** Without one the window is a search of the pages with links. With one (an address in
  `mao.ini`), the best pieces go to it and its answer is shown above them, marked as the model's.

## The sites (looked at 2026-11-12)

How they were looked at: `robots.txt`, the front page and one or two pages of each, fetched by hand with a plainly
named client (`help-index-research/0.1`), a second or more apart, about twenty-five requests in all, plus eleven
requests to AO-Universe through the crawler itself while trying it (its rules and sitemap twice, seven guides). No account, no cookies, nothing of the user.
"Allowed" below means what the site's own files say; none of the sites was asked.

| Site | What it has | robots.txt | The site's own words on reuse | Verdict |
|---|---|---|---|---|
| The game's help files (`cd_image/text/help`, 115 files, 440 KB) | Funcom's in-game help: the interface, commands, combat, death, teams, towers | none: files of the player's own install | Funcom's, part of the game the player has | **Used, on by default.** Read from the install at run time, never copied anywhere. |
| ao-universe.com | 676 guides in its sitemap (369 classic, 155 Shadowlands, 75 Alien Invasion, 29 Legacy of the Xan, 21 Lost Eden, 16 Notum Wars, 11 nostalgia), plus articles, stories and a forum | `User-agent: * / Allow: /`, closing only its admin and a few scripts; names its sitemap | Footer of every page: "Website Content may not be reproduced without permission". Legal page: a fan project, contact address of its owner, logs requests' IP addresses | **Wired up, off by default, opt-in with its terms shown.** A private copy for the player's own search is not reproduction to others; handing its text to a hosted model is a further step the player takes knowingly. Ask the team before any default-on. |
| forums.funcom.com (category "Anarchy Online", Discourse) | The official forums: thirty topics a page, about 500 posts on the first page alone | Allows topics; closes `/search`, feeds, user pages; names a sitemap | Funcom's terms of use were not read | **Not wired up.** Technically easy (Discourse serves `/c/anarchy-online/21.json` and `/t/<id>.json`), but forum posts are other players' words, uneven in worth, and Funcom's terms need reading first. |
| auno.org | The old item and nano database with comments | An empty file (nothing closed) | Its news of 2026-09-16 says the site is getting over a hundred requests a second from AI crawlers and that the owner may stop bothering to keep it up | **Not to be crawled.** The owner has said in plain words what automated traffic is doing to the site. Link to it at most. (The item data itself is in the game's own database, which the client already reads.) |
| aodb.us (and its wiki and mirrors) | A wiki and database mirrors | Allows everyone, with a content signal `search=yes, ai-train=no`, and closes every named AI crawler | The signal is the statement | **Not wired up.** Search use is offered, AI use is refused: this window does both. It could be a search-only source (never sent to a model) if that distinction is built; not in the first cut. |
| anarchy-online.fandom.com | The Fandom wiki (CC BY-SA text) | Could not be read: the site answers a bot challenge | Fandom's terms restrict automated access | **Not fetched.** A bot challenge is a no; nothing was done to get past it. |
| aoitems.com | Was the item database | Allows everyone | — | **Gone.** The address now serves an unrelated site. AO-Universe's pages still embed pictures from it. |
| www.anarchy-online.com | Funcom's game site | No robots.txt (404) | not read | Not looked at further: little help text. |
| aosetups.com, tinkeringidiot.com, aofroobs.com, aowiki.de | A setup planner (a tool, not text); the others do not answer or answer with an error | — | — | Nothing to index. |

What follows from it:

1. Only AO-Universe has, today, both a lot of player knowledge and a robots.txt that welcomes a reader. Its
   "may not be reproduced" is about publishing its content elsewhere. This design never does that (no page text in
   the repository, in a package, or sent to anyone but, when the player sets one up, the player's chosen model), but
   it is the site owner's call whether a tool that makes local copies for 676 pages is welcome. **Recommendation:
   write to the AO-Universe team (the address is on their legal page) before the client ships with this visible;
   until an answer, the source stays opt-in with their sentence shown beside the checkbox.**
2. Small sites kept up by one person are being hurt by crawlers right now (auno says so). The crawler here is built
   to be the opposite of that: one page every two seconds or slower, only pages the sitemap lists, only new or
   changed ones after the first time, a plain name, robots.txt obeyed to the letter, and it runs only when a player
   presses a button. A full first reading of AO-Universe's guides is 676 requests over about 25 minutes, once.
3. Sites that refuse AI use (aodb.us) or automated access (Fandom) are left alone.

### How AO-Universe's pages are built (what the reader relies on)

- `https://www.ao-universe.com/sitemap.xml`: 959 `<url>` entries with `<loc>` and `<lastmod>`; the guides are the
  676 under `/guides/<expansion>/<category>/<name>`.
- A guide's text is inside the page's one `<article>`: an `<h1>` title, a contents box (`div.tbox.toc`), an info box
  (class, faction, level, an item-link picker, a feedback link), then the text in a forum's markup: no heading tags;
  each part begins at `<a name="tocN"></a>` followed by a "to the top" arrow (`div.bbtotop`) and the part's heading in
  a sized, bold span, up to the next line break. Tables are plain `<table>`s.
- The `<title>` is "Name • AO-Universe".

`mao_help::text::extract` reads that with general rules (take the `<article>`; drop scripts, menus, forms, pickers
and anything classed as contents or navigation; start a new part at a heading tag or at a `toc` anchor), so nothing
in it is written for this one site but the anchor rule, and other sites' articles read the same way.

## Design

```
                      (the player's machine)
  installed game ──► game help pages ─┐
                                      ├─► pieces ─► BM25 index (in memory) ─► results (title, lines, link)
  a site, when turned on:             │                                   └─► optional: model's answer from the
    robots.txt ─► sitemap ─► pages ─► help/<site>.jsonl                        top pieces, sources numbered
          (slow, by the site's rules)
```

Code: the crate `crates/mao-help` (no dependency of its own beyond `serde`/`serde_json`, which the workspace already
had), a command-line program `mao-help` for trying it, and the window in `crates/mao-client/src/helpwindow.rs`.

### Sources

- `game`: `mao_help::game_docs` reads `cd_image/text/help/*.html` of the game folder (not the `index-*.html` link
  lists). The files are text with a few tags and meaningful line ends; a headline is a `<font color=CCInfoHeadline>`.
  Nothing is cached: 110 files are read and indexed in a few milliseconds each time.
- `crafting` (2026-11-12): `mao_help::crafting_docs` reads the game's item database through
  `mao_formats::crafting`: each book or manual, and each item whose description speaks of making, is longer than
  a line and has no bracket ending, is a page under the item's name (3,476 pages on the install looked at; about
  two seconds). On by default beside `game`. The window names such a result "the game's crafting texts (an item's
  description)". See `docs/TRADESKILL_GUIDE.md`.
- A site (`mao_help::crawl::SITES`): a name, an origin, a sitemap path, the path prefixes that hold guides, a least
  gap between requests, and the owner's terms in a sentence for the player to read. Adding a site whose guides are
  listed in a sitemap is one entry there. Sites that need another way in (a Discourse forum's JSON, a MediaWiki
  API) need a reader of their own beside `crawl::crawl`; none is written.

### Reading a site (`crawl::crawl`)

1. `GET /robots.txt`. No file (404) closes nothing; a file that cannot be had (any other failure) stops the reading:
   nothing is fetched without the rules. The rules are those of the group that names the program, else of `*`
   (`mao_help::robots`, RFC 9309: longest matching rule, `*` and `$` in patterns, `Crawl-delay`).
2. `GET` the sitemap (if robots.txt allows it), keep the addresses on the site's own origin under the guide paths.
3. For each: skipped if robots.txt closes it; kept as it is if the copy on disk has the sitemap's same `lastmod`;
   else fetched, after waiting the longer of the site's gap and its `Crawl-delay` (never under a second), read to
   text, and kept with the time and the `lastmod`.
4. Every twenty pages, and at the end or when stopped, the pages are written to `help/<site>.jsonl` beside the
   settings (a page a line: address, title, parts). A reading cut short loses at most twenty pages and goes on from
   there the next time.

The program names itself `ModernAnarchyOnline-help/0.1 (a player's own help index)` and sends nothing else: no
cookies, no account, no referrer. Requests are made by the system's `curl` program (`mao_help::fetch::Curl`):
Windows 10 and later, macOS and nearly every Linux have it, and it brings TLS without adding a TLS library to the
client (the client's own HTTP library is built without one). Headers and bodies go to curl through its standard
input and a file, never on the command line. `fetch::Http` is the seam: a built-in client can replace curl without
touching the crawler.

### The index (`mao_help::index`)

- A page's parts are cut into pieces of at most 900 characters at paragraph and sentence ends.
- A piece's words: lower case, split at anything but letters, digits and apostrophes, a short stop list, a plural's
  `s` dropped. The page's title and the part's heading count as the piece's own words, twice.
- Ranking is BM25 (k1 1.2, b 0.75). Results: at most six, at most two from one page, none worth under 30 % of the
  best; each with about 240 characters around the asked words.
- All in memory, built when the window first opens and after a site is read: AO-Universe's 676 guides are a few
  megabytes of text. No index file, no database, no embedding model.

Why not embeddings: they need a model on the machine or a service, and for a few thousand pieces of guide text where
players ask with the game's own words ("ncu", "implant", "tower"), word ranking is good enough to start with. If it
proves too literal, a small embedding model behind the same endpoint setting is the next step, as a re-ranker over
BM25's top fifty.

### The answer (`mao_help::llm`)

Off until `[help]` in `mao.ini` names an `endpoint` and a `model`. Then a question's results (as many as fit
`context_chars`, 6000 by default) are sent with the instruction to answer only from them, to say so when they do not
hold the answer, and to name the source of each statement by its number. The answer is shown above the results with
the line "Written by the model from results 1 to n; check it against them."

```ini
[help]
sources = game, ao-universe
; a model on this machine (ollama shown; llama.cpp's server and LM Studio speak the same protocol)
endpoint = http://localhost:11434/v1/chat/completions
model = <the model's name on that server>
api = openai
key_env =
context_chars = 6000
```

- `api = openai`: `POST endpoint` with `{model, messages: [system, user], temperature: 0.2, stream: false}`; the
  answer is `choices[0].message.content`. A key, if `key_env` names a variable that holds one, goes in
  `Authorization: Bearer`.
- `api = anthropic`: Anthropic's Messages API: `endpoint = https://api.anthropic.com/v1/messages`, `model` a Claude
  model, `key_env = ANTHROPIC_API_KEY`; the request is `{model, max_tokens, system, messages}` with the headers
  `x-api-key` and `anthropic-version: 2023-06-01`, and the answer is the text of `content`. Written and unit-tested
  against the documented shapes; **not tried against the real service**.
- The key is never written to `mao.ini`: the file names the environment variable.
- What leaves the machine with a hosted endpoint: the question and the text of the top results. With a turned-on
  site that means pieces of that site's guides go to the model's provider. The window says which model answers; the
  settings' comment and this document say the rest. A model on the player's own machine sends nothing anywhere.

Which small model: not tried here (no model server on the development machine). The task is easy for a model (read
five short passages, answer in two sentences with numbers), so the 3 to 4 billion parameter class that runs on a
gaming machine's spare memory is the place to start: for the user to choose and try.

### The window (`helpwindow.rs`)

- A question field and Ask (Enter asks). Results numbered, each with its title and part, where it is from, a few
  lines; a click on the title shows the whole piece; "open the page" opens a site's page in the system's browser
  (only addresses of the sites the client reads).
- "Sources and the model": a checkbox for the game's help, one for each known site with the owner's terms under it
  and, when on, "Read its guides now" (with the progress and Stop), how many pages are searched, and which model
  answers or that none is set.
- Opened with `/open assistant` or the Help menu's "Ask the guides". No key of its own yet.
- The window keeps its own state and settings behind locks so that the shared files change by a handful of lines:
  `ui.rs` (two names in `window_open` / `set_window`, the draw call, the preview's draw call), `settings.rs` (read
  and write the `[help]` section), `vitals.rs` (the menu entry), `main.rs` (`mod helpwindow`), the client's
  `Cargo.toml` (the dependency).

### What is deliberately not here

- No page text in the repository, the tests, the packages or this document.
- No reading of a site the player has not turned on, and none on its own: only at the button.
- No account, character or game data in any request. (The assistant does not know who is playing.)
- No bypassing of bot challenges, logins or robots.txt.

## The first cut: what works and how it was checked

| Part | State | Checked by |
|---|---|---|
| Text out of HTML (`text.rs`) | works | unit tests on made-up pages; seven real AO-Universe guides read through it by hand (titles, parts and headings right; the info box's lines stay in the first piece) |
| robots.txt (`robots.rs`) | works | unit tests (groups, longest match, `*`, `$`, delay, named program) |
| Crawler (`crawl.rs`) | works | unit tests against a made-up web (rules obeyed, gaps, unchanged pages kept, limit, stop, no robots / unreadable robots); `mao-help fetch ao-universe --limit 4` and `--limit 3` against the real site |
| Index and ranking (`index.rs`) | works | unit tests; the game's 110 help pages (`MAO_GAME` test: a question about dying finds the page on death and insurance) |
| The game's help as a source | works | the same test; `mao-help ask` by hand |
| Command line (`mao-help`) | works | by hand: `sites`, `fetch`, `forget`, `stats`, `ask`, `ask --answer` |
| The answer step, OpenAI protocol | works against a stand-in | unit tests; `mao-help ask --answer` against a made-up endpoint on this machine through the real `curl` (path, headers, key, body arrive; the answer is read) |
| The answer step, Anthropic protocol | written | unit tests on the request's and the answer's shapes only |
| The window | draws | `MAO_UI_PREVIEW=assistant MAO_HELP_QUESTION=... mao-client <game> map 540 --image out.png` (results from the game's help and from the pages fetched while trying) |
| The window in the live game, typing, the buttons, reading a site from it, a real model | **not tried** | — |
| Linux and macOS (`curl`, `xdg-open` / `open`) | **not tried** | — |

Trying it:

```sh
cargo run --release -p mao-help -- ask --game "<game folder>" how do I get insurance
cargo run --release -p mao-help -- sites
cargo run --release -p mao-help -- fetch ao-universe --limit 5     # five pages, into the settings' help folder
```

## The model inside the client (2026-11-12)

The user's answer to "which model, and where": "We need a small llm, only needs to interact with our data really."
So the client carries the means to run one itself, and the model sees nothing but the question and the pieces of the
player's index found for it.

**What runs it.** `candle` (Hugging Face's inference library, written in Rust) and the `tokenizers` crate, in
`mao-help` behind the `local-model` feature, which is on by default (`crates/mao-help/src/local.rs`). No other
program to install, no C++ toolchain to build (llama.cpp's Rust binding was the other candidate: faster, but it needs
libclang and CMake on every build machine, which this one does not have). The client's program grew from 29.9 to
36.7 MB.

**Which model.** Qwen2.5 1.5B Instruct, quantized (`q4_k_m`, 1,117 MB), from the Qwen team's own repository on
Hugging Face; Apache-2.0, not gated. Tried against the same family's 0.5B size (491 MB) on this machine the same day:

| | 1.5B | 0.5B |
|---|---|---|
| A question its source answers | answered from the source | answered "[1]" and stopped with the large model's instructions; answered with ours |
| A question its source does not answer | "The sources do not say." | made an answer up (a price, opening hours as a price) |
| "What happens when I die" over the game's help | the bank, the new body, the experience pool: right | "your experience points are lost": wrong |
| Reading about 700 tokens of sources, then writing | 2.6 s, then about 13 tokens a second (32 processor threads here, 8 used) | slower to read on this machine (12 s), for no reason found |

So the 0.5B size is not offered. The model is not in the repository and not in a package: `local::MODELS` names the
two files, where they come from, the weights' size and SHA-256, and the client fetches them when the player presses
the button (or `mao-help model fetch`), to `<help folder>/models/qwen2.5-1.5b/`. The fetch goes on from where a
broken one stopped; a file of the wrong size or the wrong hash is not used.

**How it is asked** (`local::ask`, `SYSTEM`, `RULE`). The chat form of the family; the system's part says that it
knows nothing but the numbered sources; then the sources and the question; then, last, the rule again: one to three
short sentences from the sources only, and exactly "The sources do not say." when they do not hold the answer. The
rule at the end is what made the 1.5B model stop inventing: with the same rule only in the system's part it gave a
room's price that its source never named. The likeliest token is taken each time (the same question over the same
sources gives the same answer), with a light penalty on repeating, up to 220 tokens. At most 5,000 characters of
sources go in.

**In the window.** "Write an answer in words with the small model inside the client" in the window's "Sources and
the model"; ticked without the files, a button fetches them and says how far it is. The model is loaded with the
first question (about 2 s here), kept for five minutes after its last answer and then let go (it holds about a
gigabyte of memory). It works on its own few threads (half the machine's, between 2 and 8), so the game keeps
drawing: tried live, the frame rate went from about 195 to between 120 and 150 while it answered (6 s), with no
frame longer than 8 ms. The answer appears as it is written, above the results, with the line that it is the
model's and is to be checked against them. `[help] local_model` in `mao.ini` keeps the choice; it comes before a
model behind an address when both are set.

**What it gets wrong.** It can only be as right as the pieces found. Asked how to invite someone to a team, the
search's best pieces were the chat commands and the organization's commands, and the model answered with the
organization's invitation: the game's help says little about inviting to a team, and a small model takes the nearest
thing. The answer always stands over the results it was written from, for that reason.

**"Our data"** is, today, the game's own help pages (and a site's guides where the player turned one on). The next
sources worth adding, each a matter of making `Doc`s of it: the items and nano programs of the game's database (names,
descriptions, requirements), the player's own item index (`itemindex.rs`: "where is my ..."), and the missions and
perks the client already reads.

## Open questions for the user

1. **AO-Universe's permission.** Ask its team before this ships visible to players? (Recommended: yes. Their
   content is the heart of this, and their footer asks for permission.) Until then it stays opt-in.
2. **Which sources by default.** The first cut: the game's help on, everything else off.
3. **Which model, and where.** Answered 2026-11-12: a small model inside the client, working only on our data (the
   section above). The model's files are fetched only when the player asks for them. A model behind an address
   stays possible for whoever wants a larger one.
4. **Forums.** Funcom's forums are readable by their robots.txt; do you want them, given they are other players'
   posts? Their terms of use need reading first.
5. **Search-only sources.** aodb.us offers itself for search but not for AI use. Worth building the distinction
   (a source that is listed but never sent to a model)?
6. **A key for the window**, and whether the old Help window (Ctrl+H: keys and commands) and this one should become
   one.

## Next steps (not done)

- A setting-less first run: the window offers the sources with their terms the first time it opens.
- Reading in the background on a schedule (weekly, changed pages only) instead of only at the button; still slow.
- A reader for Discourse forums and one for MediaWiki, if those sources are wanted and allowed.
- Item, nano and playfield names from the game's database as their own source ("where does X drop" needs data, not
  prose), and links from results into the client's own windows (the info view, the map).
- Showing a game help page whole in the client's info view, with its links working.
- A built-in HTTP client instead of `curl`, if a TLS library is ever added to the client.
