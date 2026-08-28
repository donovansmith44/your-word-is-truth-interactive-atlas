//! `atlas-cli`: a really simple, offline binary that queries the Bible
//! Explorer graph directly -- no server, no HTTP (design ruling R1). Args
//! are hand-parsed (R3, the `atlas-server/src/main.rs` precedent -- no
//! clap, no new runtime deps). See `CONTRACT.md` for the full command
//! vocabulary, error taxonomy, and tutorial contract this binary
//! implements; `report.md`'s own "self-review" section is checked against
//! that document line by line.

mod commands;
mod error;
mod load;

use std::path::PathBuf;

use error::CliError;

fn main() {
    let raw: Vec<String> = std::env::args().skip(1).collect();
    match run(&raw) {
        Ok(output) => {
            print!("{output}");
        }
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(e.exit_code());
        }
    }
}

/// Pulls `--data-dir <path>` out of `args` wherever it appears (before or
/// after the subcommand -- a global flag, not positional) and returns the
/// resolved path plus the remaining args in their original relative order.
fn extract_data_dir(args: &[String]) -> Result<(PathBuf, Vec<String>), CliError> {
    let mut data_dir: Option<PathBuf> = None;
    let mut rest = Vec::with_capacity(args.len());
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--data-dir" {
            i += 1;
            let v = args.get(i).ok_or_else(|| CliError::bad_usage("--data-dir requires a value", "no path followed --data-dir", "pass a directory, e.g. --data-dir ../data/compiled"))?;
            data_dir = Some(PathBuf::from(v));
        } else {
            rest.push(args[i].clone());
        }
        i += 1;
    }
    Ok((data_dir.unwrap_or_else(load::default_data_dir), rest))
}

fn run(args: &[String]) -> Result<String, CliError> {
    let (data_dir, rest) = extract_data_dir(args)?;

    let Some(cmd) = rest.first() else {
        return Ok(commands::help::text());
    };

    match cmd.as_str() {
        "help" => Ok(commands::help::text()),
        "tutorial" => {
            let loaded = load::load(&data_dir)?;
            commands::tutorial::run(&loaded.graph, &loaded.data, &data_dir.display().to_string())
        }
        "verse" => {
            let ref_raw = one_positional(&rest, "verse", "<ref>")?;
            let loaded = load::load(&data_dir)?;
            commands::verse::run(&loaded.graph, &loaded.data, ref_raw)
        }
        "chapter" => {
            let ref_raw = one_positional(&rest, "chapter", "<ref>")?;
            let loaded = load::load(&data_dir)?;
            commands::chapter::run(&loaded.graph, ref_raw)
        }
        "node" => {
            let id_raw = one_positional(&rest, "node", "<id>")?;
            let loaded = load::load(&data_dir)?;
            commands::node::run(&loaded.graph, id_raw)
        }
        "edges" => {
            let (id_raw, kind_raw, limit, cursor) = parse_edges_args(&rest)?;
            let loaded = load::load(&data_dir)?;
            commands::edges::run(&loaded.graph, commands::edges::EdgesArgs { id_raw, kind_raw, limit, cursor })
        }
        "find" => {
            let term = one_positional(&rest, "find", "<term>")?;
            let loaded = load::load(&data_dir)?;
            commands::find::run(&loaded.graph, term)
        }
        other => Err(CliError::bad_usage(
            format!("unrecognized subcommand '{other}'"),
            "'atlas' only knows verse, chapter, node, edges, find, tutorial, help",
            "run 'atlas help' for the full list",
        )),
    }
}

/// Every single-positional-argument command (`verse`/`chapter`/`node`/
/// `find`) shares this exact validation: exactly one argument after the
/// subcommand name, nonempty.
fn one_positional<'a>(rest: &'a [String], cmd: &str, shape: &str) -> Result<&'a str, CliError> {
    match rest.len() {
        1 => Err(CliError::bad_usage(format!("'{cmd}' requires an argument"), format!("usage: atlas {cmd} {shape}"), format!("run 'atlas {cmd} {shape}' with a real value, or 'atlas tutorial' for a worked example"))),
        2 => Ok(rest[1].as_str()),
        _ => Err(CliError::bad_usage(
            format!("'{cmd}' takes exactly one argument, got {}", rest.len() - 1),
            format!("usage: atlas {cmd} {shape}"),
            "quote a multi-word argument (e.g. \"BoC 7.2.1\") so it arrives as one shell word",
        )),
    }
}

#[allow(clippy::type_complexity)]
fn parse_edges_args(rest: &[String]) -> Result<(&str, Option<&str>, Option<usize>, Option<usize>), CliError> {
    let mut positionals: Vec<&str> = Vec::new();
    let mut kind_raw: Option<&str> = None;
    let mut limit: Option<usize> = None;
    let mut cursor: Option<usize> = None;

    let mut i = 1; // skip "edges" itself
    while i < rest.len() {
        match rest[i].as_str() {
            "--kind" => {
                i += 1;
                let v = rest.get(i).ok_or_else(|| CliError::bad_usage("--kind requires a value", "no edge-kind label followed --kind", "pass a label, e.g. --kind cites (see 'atlas node <id>' for the labels a given node carries)"))?;
                kind_raw = Some(v.as_str());
            }
            "--limit" => {
                i += 1;
                let v = rest.get(i).ok_or_else(|| CliError::bad_usage("--limit requires a value", "no number followed --limit", "pass a positive integer, e.g. --limit 50"))?;
                limit = Some(v.parse().map_err(|_| CliError::bad_usage(format!("--limit value '{v}' is not a valid number"), "expected a positive integer", "pass e.g. --limit 50"))?);
            }
            "--cursor" => {
                i += 1;
                let v = rest.get(i).ok_or_else(|| CliError::bad_usage("--cursor requires a value", "no number followed --cursor", "pass the integer a previous page's own 'more: continue with --cursor N' line printed"))?;
                cursor = Some(v.parse().map_err(|_| CliError::bad_usage(format!("--cursor value '{v}' is not a valid number"), "expected a nonnegative integer", "pass the exact number a previous page's own 'more: continue with --cursor N' line printed"))?);
            }
            other if other.starts_with("--") => {
                return Err(CliError::bad_usage(format!("unrecognized flag '{other}' for 'edges'"), "'edges' only accepts --kind, --limit, and --cursor", "run 'atlas help' or 'atlas tutorial' for the full shape"));
            }
            positional => positionals.push(positional),
        }
        i += 1;
    }

    match positionals.len() {
        0 => Err(CliError::bad_usage("'edges' requires an <id> argument", "usage: atlas edges <id> --kind K [--limit N] [--cursor C]", "run 'atlas edges <id> --kind K' with a real id, or 'atlas tutorial' for a worked example")),
        1 => Ok((positionals[0], kind_raw, limit, cursor)),
        _ => Err(CliError::bad_usage(format!("'edges' takes exactly one <id> argument, got {}", positionals.len()), "usage: atlas edges <id> --kind K [--limit N] [--cursor C]", "quote a multi-word id if it has one, and check no flag value was left unconsumed")),
    }
}
