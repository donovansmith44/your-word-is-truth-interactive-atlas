//! `atlas-cli`: a really simple, offline binary that queries the Bible
//! Explorer graph directly -- no server, no HTTP (design ruling R1). Args
//! are hand-parsed (R3, the `atlas-server/src/main.rs` precedent -- no
//! clap, no new runtime deps). See `CONTRACT.md` for the full command
//! vocabulary, error taxonomy, tutorial contract, `--json` mode, and ID
//! discoverability conventions this binary implements; `report.md`'s own
//! "self-review" section is checked against that document line by line.

mod commands;
mod error;
mod load;

use std::path::PathBuf;

use error::CliError;

fn main() {
    let raw: Vec<String> = std::env::args().skip(1).collect();
    // BIBEX-1 (--json mode): whether `--json` is present must be known
    // BEFORE any fallible parsing runs, so a bad-usage error that occurs
    // while EXTRACTING the other global flags (e.g. a dangling
    // `--data-dir` with no value) still gets rendered the right way --
    // a pure presence check over the raw, unparsed args, which can never
    // itself fail. `--json` is then stripped here (once), so neither
    // `run` nor `run_json` ever sees it as a stray token.
    let json = raw.iter().any(|a| a == "--json");
    let stripped: Vec<String> = if json { raw.iter().filter(|a| a.as_str() != "--json").cloned().collect() } else { raw };

    if json {
        match run_json(&stripped) {
            Ok(value) => {
                println!("{}", serde_json::to_string(&value).expect("every value this crate builds is a plain, already-valid JSON tree"));
            }
            Err(e) => {
                eprintln!("{}", serde_json::to_string(&e.to_json()).expect("CliError::to_json always produces a plain, already-valid JSON tree"));
                std::process::exit(e.exit_code());
            }
        }
    } else {
        match run(&stripped) {
            Ok(output) => {
                print!("{output}");
            }
            Err(e) => {
                eprintln!("{e}");
                std::process::exit(e.exit_code());
            }
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

/// Shared by `run` and `run_json` -- `find`'s own argument validation
/// (CONTRACT.md's own "bibex find" section: the zero-argument message
/// specifically states the widened kind-coverage scope, per review S-4 and
/// BIBEX-1 addendum ticket 2's widening of that same scope).
fn parse_find_term(rest: &[String]) -> Result<&str, CliError> {
    match rest.len() {
        1 => Err(CliError::bad_usage(
            "'find' requires a <term> argument",
            format!("usage: bibex find <term> -- searches {} labels only ({})", commands::find::SEARCHED_KINDS, commands::find::EXCLUDED_KINDS),
            "run 'bibex find <term>' with a real value, or 'bibex tutorial' for a worked example",
        )),
        2 if rest[1].starts_with("--") => {
            Err(CliError::bad_usage(format!("unrecognized flag '{}' for 'find'", rest[1]), "'find' takes a single <term> argument, not flags", "run 'bibex find <term>' with a real value, no flags"))
        }
        2 => Ok(rest[1].as_str()),
        _ => Err(CliError::bad_usage(
            format!("'find' takes exactly one argument, got {}", rest.len() - 1),
            "usage: bibex find <term>",
            "quote a multi-word term (e.g. \"the Red Sea\") so it arrives as one shell word",
        )),
    }
}

/// Shared by `run` and `run_json` -- `kinds` takes no arguments at all.
fn check_no_kinds_args(rest: &[String]) -> Result<(), CliError> {
    if rest.len() > 1 {
        return Err(CliError::bad_usage(format!("'kinds' takes no arguments, got {}", rest.len() - 1), "usage: bibex kinds", "run 'bibex kinds' with no arguments"));
    }
    Ok(())
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
            let term = parse_find_term(&rest)?;
            let loaded = load::load(&data_dir)?;
            commands::find::run(&loaded.graph, &loaded.data, term)
        }
        "kinds" => {
            check_no_kinds_args(&rest)?;
            Ok(commands::kinds::run())
        }
        other => Err(CliError::bad_usage(
            format!("unrecognized subcommand '{other}'"),
            "'atlas' only knows verse, chapter, node, edges, find, kinds, tutorial, help",
            "run 'bibex help' for the full list",
        )),
    }
}

/// BIBEX-1 (--json mode): the `--json` sibling of `run`. `tutorial`/`help`/
/// a bare invocation are `bad_usage` under `--json` (CONTRACT.md, binding
/// ruling: "a tutorial is prose by nature; say so in the message's hint")
/// -- every real query command (`verse`/`chapter`/`node`/`edges`/`find`/
/// `kinds`) gets a real JSON answer instead.
fn run_json(args: &[String]) -> Result<serde_json::Value, CliError> {
    let (data_dir, rest) = extract_data_dir(args)?;

    let Some(cmd) = rest.first() else {
        return Err(CliError::bad_usage(
            "a bare invocation has no --json output",
            "a bare invocation prints a prose help block, not a machine-shaped answer",
            "run 'bibex' or 'bibex help' without --json, or use --json with a real query command (verse/chapter/node/edges/find/kinds)",
        ));
    };

    match cmd.as_str() {
        "help" => Err(CliError::bad_usage(
            "'help' has no --json output",
            "help is a prose command list, not a machine-shaped answer",
            "run 'bibex help' without --json, or use --json with a real query command (verse/chapter/node/edges/find/kinds)",
        )),
        "tutorial" => Err(CliError::bad_usage(
            "'tutorial' has no --json output",
            "a tutorial is a guided prose walkthrough by nature, not a machine-shaped answer",
            "run 'bibex tutorial' without --json, or use --json with a real query command (verse/chapter/node/edges/find/kinds)",
        )),
        "verse" => {
            let ref_raw = one_positional(&rest, "verse", "<ref>")?;
            let loaded = load::load(&data_dir)?;
            commands::verse::run_json(&loaded.graph, &loaded.data, ref_raw)
        }
        "chapter" => {
            let ref_raw = one_positional(&rest, "chapter", "<ref>")?;
            let loaded = load::load(&data_dir)?;
            commands::chapter::run_json(&loaded.graph, ref_raw)
        }
        "node" => {
            let id_raw = one_positional(&rest, "node", "<id>")?;
            let loaded = load::load(&data_dir)?;
            commands::node::run_json(&loaded.graph, id_raw)
        }
        "edges" => {
            let (id_raw, kind_raw, limit, cursor) = parse_edges_args(&rest)?;
            let loaded = load::load(&data_dir)?;
            commands::edges::run_json(&loaded.graph, commands::edges::EdgesArgs { id_raw, kind_raw, limit, cursor })
        }
        "find" => {
            let term = parse_find_term(&rest)?;
            let loaded = load::load(&data_dir)?;
            commands::find::run_json(&loaded.graph, &loaded.data, term)
        }
        "kinds" => {
            check_no_kinds_args(&rest)?;
            Ok(commands::kinds::run_json())
        }
        other => Err(CliError::bad_usage(
            format!("unrecognized subcommand '{other}'"),
            "'atlas' only knows verse, chapter, node, edges, find, kinds, tutorial, help",
            "run 'bibex help' for the full list",
        )),
    }
}

/// Every single-positional-argument command (`verse`/`chapter`/`node`)
/// shares this exact validation: exactly one argument after the
/// subcommand name, nonempty. (`find` has its own copy in `run` above,
/// per review S-4 -- it names its own kind-coverage scope in the
/// zero-argument message, which this generic helper cannot express.)
///
/// FIX ROUND 1 (review T-3): an unrecognized `--flag` used to fall
/// through into the generic "too many arguments" message without ever
/// naming the flag -- inconsistent with `edges.rs`'s own diagnostics,
/// which always name the offending flag. Both the exactly-one-extra-arg
/// case and the too-many-args case now check for a leading `--` first and
/// name it explicitly, matching `edges`'s own message shape.
fn one_positional<'a>(rest: &'a [String], cmd: &str, shape: &str) -> Result<&'a str, CliError> {
    match rest.len() {
        1 => Err(CliError::bad_usage(format!("'{cmd}' requires an argument"), format!("usage: atlas {cmd} {shape}"), format!("run 'atlas {cmd} {shape}' with a real value, or 'bibex tutorial' for a worked example"))),
        2 if rest[1].starts_with("--") => {
            Err(CliError::bad_usage(format!("unrecognized flag '{}' for '{cmd}'", rest[1]), format!("'{cmd}' takes a single positional argument, not flags"), format!("run 'atlas {cmd} {shape}' with a real value, no flags")))
        }
        2 => Ok(rest[1].as_str()),
        _ => {
            if let Some(flag) = rest[1..].iter().find(|a| a.starts_with("--")) {
                return Err(CliError::bad_usage(format!("unrecognized flag '{flag}' for '{cmd}'"), format!("'{cmd}' takes a single positional argument, not flags"), format!("run 'atlas {cmd} {shape}' with a real value, no flags")));
            }
            Err(CliError::bad_usage(
                format!("'{cmd}' takes exactly one argument, got {}", rest.len() - 1),
                format!("usage: atlas {cmd} {shape}"),
                "quote a multi-word argument (e.g. \"BoC 7.2.1\") so it arrives as one shell word",
            ))
        }
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
                let v = rest.get(i).ok_or_else(|| CliError::bad_usage("--kind requires a value", "no edge-kind label followed --kind", "pass a label, e.g. --kind cites (see 'bibex node <id>' for the labels a given node carries)"))?;
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
                return Err(CliError::bad_usage(format!("unrecognized flag '{other}' for 'edges'"), "'edges' only accepts --kind, --limit, and --cursor", "run 'bibex help' or 'bibex tutorial' for the full shape"));
            }
            positional => positionals.push(positional),
        }
        i += 1;
    }

    match positionals.len() {
        0 => Err(CliError::bad_usage("'edges' requires an <id> argument", "usage: bibex edges <id> --kind K [--limit N] [--cursor C]", "run 'bibex edges <id> --kind K' with a real id, or 'bibex tutorial' for a worked example")),
        1 => Ok((positionals[0], kind_raw, limit, cursor)),
        _ => Err(CliError::bad_usage(format!("'edges' takes exactly one <id> argument, got {}", positionals.len()), "usage: bibex edges <id> --kind K [--limit N] [--cursor C]", "quote a multi-word id if it has one, and check no flag value was left unconsumed")),
    }
}
