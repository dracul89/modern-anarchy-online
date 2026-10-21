//! `mao-db <game dir> info | verify | get <type> <instance> (hex) | count-types`

use std::collections::BTreeMap;
use std::path::Path;
use std::process::ExitCode;

use mao_formats::resource::{Id, ResourceDatabase};

/// A hexadecimal number, with or without `0x`.
fn number(text: &str) -> Option<u32> {
    u32::from_str_radix(text.strip_prefix("0x").unwrap_or(text), 16).ok()
}

fn run(args: &[String]) -> Result<(), String> {
    let [game, command, rest @ ..] = args else {
        return Err("usage: mao-db <game dir> info | verify | get <type> <instance> (hex) | count-types".into());
    };
    let db = ResourceDatabase::open_game(Path::new(game)).map_err(|e| e.to_string())?;
    match (command.as_str(), rest) {
        ("info", []) => println!("{} objects", db.len()),
        ("get", [kind, instance]) => {
            let id = Id { kind: number(kind).ok_or("bad type")?, instance: number(instance).ok_or("bad instance")? };
            match db.get(id).map_err(|e| e.to_string())? {
                Some(data) => {
                    println!("{} bytes", data.len());
                    let head: Vec<String> = data.iter().take(32).map(|b| format!("{b:02x}")).collect();
                    println!("{}", head.join(" "));
                }
                None => return Err("no such object".into()),
            }
        }
        ("verify", []) => {
            let (count, bytes, highest, failures, failed) = db.verify().map_err(|e| e.to_string())?;
            println!("{count} objects, {bytes} bytes, highest position {highest:#x}, {failures} unreadable");
            for (id, position, why) in failed {
                println!("failed {:#x}:{:#x} at {position:#x}: {why}", id.kind, id.instance);
            }
        }
        ("count-types", []) => {
            let mut counts: BTreeMap<u32, u64> = BTreeMap::new();
            for id in db.ids().map_err(|e| e.to_string())? {
                *counts.entry(id.map_err(|e| e.to_string())?.kind).or_default() += 1;
            }
            for (kind, count) in counts {
                println!("{kind:#x}: {count}");
            }
        }
        _ => return Err("unknown command".into()),
    }
    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("mao-db: {e}");
            ExitCode::FAILURE
        }
    }
}
