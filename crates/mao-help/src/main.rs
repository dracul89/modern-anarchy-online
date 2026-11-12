//! `mao-help`: the help assistant from the command line, for trying it without the client.
//!
//! ```text
//! mao-help sites                                   the sites that can be read, and what their owners say
//! mao-help fetch <site> [--limit N] [--dir D]      read up to N pages of a site into the help folder (default 20)
//! mao-help forget <site> [--dir D]                 drop a site's kept pages
//! mao-help ask [--game G] [--sources a,b] [--dir D] [--answer] <question...>
//!                                                  the best pieces for a question; with --answer, the model's answer
//! mao-help stats [--game G] [--sources a,b] [--dir D]   how many pages and pieces the index has
//! ```
//!
//! `--game` (or `MAO_GAME`) is the installed game's folder, for the game's own help files; `--dir` the help folder
//! (default: `help` beside the client's settings). The model for `--answer` comes from the environment here:
//! `MAO_HELP_ENDPOINT`, `MAO_HELP_MODEL`, `MAO_HELP_API` (`openai` or `anthropic`) and `MAO_HELP_KEY_ENV` (the name of
//! the variable that holds the key); the client reads the same from `mao.ini`'s `[help]` section.

use std::path::PathBuf;
use std::process::ExitCode;

use mao_help::crawl::{self, Step};
use mao_help::fetch::Curl;
use mao_help::index::{default_dir, Store};
use mao_help::llm::{self, Api, Model};

struct Args {
    dir: PathBuf,
    game: Option<PathBuf>,
    sources: Vec<String>,
    limit: usize,
    answer: bool,
    /// `--local <folder>`: answer with the model inside the client, its `model.gguf` and `tokenizer.json` in the folder.
    local: Option<PathBuf>,
    rest: Vec<String>,
}

fn args() -> Result<Args, String> {
    let mut out = Args { dir: default_dir(), game: std::env::var_os("MAO_GAME").map(PathBuf::from), sources: Vec::new(), limit: 20, answer: false, local: None, rest: Vec::new() };
    let mut given = std::env::args().skip(1);
    while let Some(arg) = given.next() {
        let mut value = |name: &str| given.next().ok_or(format!("{name} needs a value"));
        match arg.as_str() {
            "--dir" => out.dir = PathBuf::from(value("--dir")?),
            "--game" => out.game = Some(PathBuf::from(value("--game")?)),
            "--sources" => out.sources = mao_help::source_names(&value("--sources")?),
            "--limit" => out.limit = value("--limit")?.parse().map_err(|_| "--limit needs a number")?,
            "--answer" => out.answer = true,
            "--local" => out.local = Some(PathBuf::from(value("--local")?)),
            _ => out.rest.push(arg),
        }
    }
    Ok(out)
}

fn now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

fn run() -> Result<(), String> {
    let args = args()?;
    let store = Store::new(&args.dir);
    // (without --sources: the game's help and every site that has pages kept)
    let sources = if args.sources.is_empty() { [mao_help::GAME.to_string(), mao_help::CRAFTING.to_string()].into_iter().chain(store.sources()).collect() } else { args.sources.clone() };
    let command = args.rest.first().map(String::as_str).unwrap_or("");
    match command {
        "sites" => {
            for site in crawl::SITES {
                println!("{}: {} ({}{})\n  {}\n  kept: {} pages", site.name, site.label, site.origin, site.include.join(", "), site.terms, store.load(site.name).len());
            }
            Ok(())
        }
        "fetch" => {
            let name = args.rest.get(1).ok_or("fetch needs a site's name (see `mao-help sites`)")?;
            let site = crawl::site(name).ok_or(format!("no site called {name:?} (see `mao-help sites`)"))?;
            if !Curl::available() {
                return Err("the curl program is not there to fetch with".into());
            }
            println!("{}", site.terms);
            let mut http = Curl::new(&args.dir);
            let known = store.load(site.name);
            let (docs, report) = crawl::crawl(&mut http, site, known, args.limit, now(), &mut |step| match step {
                Step::Fetching(address, n, of) => println!("{n}/{of} {address}"),
                Step::Checkpoint(docs) => {
                    let _ = store.save(site.name, docs);
                }
            }, &|| false)?;
            store.save(site.name, &docs).map_err(|e| e.to_string())?;
            println!("{}: {} pages kept in {} ({} listed, {} fetched now, {} unchanged, {} closed by robots.txt, {} failed, {} left for another run)", site.label, docs.len(), args.dir.display(), report.listed, report.fetched, report.kept, report.closed, report.failed, report.left);
            Ok(())
        }
        "forget" => {
            let name = args.rest.get(1).ok_or("forget needs a site's name")?;
            store.forget(name).map_err(|e| e.to_string())
        }
        // `model`: the model inside the client, whether its files are here; `model fetch` gets them (once, about a
        // gigabyte), going on from a fetch that was broken off.
        #[cfg(feature = "local-model")]
        "model" => {
            let known = mao_help::local::known(args.rest.get(2).map(String::as_str).unwrap_or(""));
            if args.rest.get(1).map(String::as_str) == Some("fetch") {
                if !Curl::available() {
                    return Err("the curl program is not there to fetch with".into());
                }
                println!("{}: fetching from {}", known.title, known.weights_url);
                mao_help::local::fetch_model(&mut Curl::new(&args.dir), known, &args.dir, &std::sync::atomic::AtomicBool::new(false))?;
            }
            println!("{}: {} ({} of {} bytes in {})", known.title, if known.present(&args.dir) { "here" } else { "not here" }, mao_help::local::fetched_bytes(known, &args.dir), known.weights_bytes, known.dir(&args.dir).display());
            Ok(())
        }
        "stats" => {
            let index = mao_help::load(&store, args.game.as_deref(), &sources);
            println!("{} pages, {} pieces from {:?} (help folder {})", index.docs.len(), index.chunks.len(), sources, args.dir.display());
            Ok(())
        }
        "ask" => {
            let question = args.rest[1..].join(" ");
            if question.trim().is_empty() {
                return Err("ask needs a question".into());
            }
            let index = mao_help::load(&store, args.game.as_deref(), &sources);
            let hits = index.search(&question, 6);
            if hits.is_empty() {
                println!("Nothing in the index ({} pages) matches.", index.docs.len());
                return Ok(());
            }
            for (n, hit) in hits.iter().enumerate() {
                let heading = if hit.heading.is_empty() || hit.heading == hit.title { String::new() } else { format!(" - {}", hit.heading) };
                println!("[{}] {}{} ({}: {}, {:.1})\n    {}\n", n + 1, hit.title, heading, hit.source, hit.url, hit.score, hit.snippet);
            }
            // (`--local` with a folder of the two files; `--local here` for the model kept in the help folder)
            #[cfg(feature = "local-model")]
            if let Some(dir) = &args.local {
                let dir = if dir.as_os_str() == "here" { mao_help::local::MODELS[0].dir(&args.dir) } else { dir.clone() };
                let dir = &dir;
                let started = std::time::Instant::now();
                let mut model = mao_help::local::Local::load(&dir.join("model.gguf"), &dir.join("tokenizer.json"))?;
                let loaded = started.elapsed();
                let chars = std::env::var("MAO_HELP_CONTEXT").ok().and_then(|v| v.parse().ok()).unwrap_or(5000);
                let (user, _) = mao_help::local::ask(&question, &hits, chars);
                let prompt_tokens = model.tokens(&mao_help::local::chat(mao_help::local::SYSTEM, &user));
                let asked = std::time::Instant::now();
                let mut first = None;
                let answer = mao_help::local::answer(&mut model, &question, &hits, chars, &mut |_| {
                    first.get_or_insert_with(|| asked.elapsed());
                    true
                })?;
                println!("Answer (the model in the client, from sources 1 to {}):\n{}", answer.sources, answer.text);
                println!("[loaded in {:.1} s; {} prompt tokens read in {:.1} s; the answer's {} tokens in {:.1} s more]", loaded.as_secs_f32(), prompt_tokens, first.unwrap_or_default().as_secs_f32(), model.tokens(&answer.text), (asked.elapsed() - first.unwrap_or_default()).as_secs_f32());
            }
            if args.answer {
                let env = |name: &str| std::env::var(name).unwrap_or_default();
                let model = Model { api: if env("MAO_HELP_API").eq_ignore_ascii_case("anthropic") { Api::Anthropic } else { Api::OpenAi }, endpoint: env("MAO_HELP_ENDPOINT"), model: env("MAO_HELP_MODEL"), key_env: env("MAO_HELP_KEY_ENV"), ..Default::default() };
                let key = (!model.key_env.is_empty()).then(|| env(&model.key_env));
                let answer = llm::answer(&mut Curl::new(&args.dir), &model, key.as_deref(), &question, &hits)?;
                println!("Answer (from sources 1 to {}):\n{}", answer.sources, answer.text);
            }
            Ok(())
        }
        _ => Err("commands: sites | fetch <site> [--limit N] | forget <site> | ask [--answer] [--local <model folder>|here] <question> | model [fetch] | stats   (options: --game <folder>, --dir <help folder>, --sources a,b)".into()),
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("mao-help: {e}");
            ExitCode::FAILURE
        }
    }
}
