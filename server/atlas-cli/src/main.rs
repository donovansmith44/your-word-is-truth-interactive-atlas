//! An offline binary that queries the graph directly: no server, no HTTP, and arguments
//! parsed by hand rather than by a dependency.

mod commands;
mod error;
mod load;

use std::path::PathBuf;

use error::CliError;

fn main() {
    let raw: Vec<String> = std::env::args().skip(1).collect();
    // Whether `--json` was passed has to be known before any fallible parsing, so that a
    // bad-usage error raised while extracting the other flags is still rendered as JSON.
    // Stripped here once, so no command ever sees it as a stray token.
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

/// A global flag, not a positional one: it may appear before or after the subcommand, and
/// the remaining arguments keep their original relative order.
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

fn parse_verify_args(rest: &[String]) -> Result<Option<String>, CliError> {
    match &rest[1..] {
        [] => Ok(None),
        [flag, name] if flag == "--section" => Ok(Some(name.clone())),
        [flag] if flag == "--section" => Err(CliError::bad_usage(
            "--section requires a value",
            "usage: bibex verify [--section <name>]",
            "name one of the manifest's sections (core, kjv, concord, kretzmann), or omit --section to verify all",
        )),
        other => Err(CliError::bad_usage(
            format!("unrecognized arguments for 'verify': {}", other.join(" ")),
            "usage: bibex verify [--section <name>]",
            "run 'bibex verify' with no arguments, or exactly '--section <name>'",
        )),
    }
}

/// The verbs under `raw`; the type is the list.
enum RawVerb {
    Bless,
    Check(String),
}

fn parse_raw_verb(rest: &[String]) -> Result<RawVerb, CliError> {
    const USAGE: &str = "usage: bibex raw bless | bibex raw check <path>";
    const DO: &str = "run 'bibex raw bless' to record data/raw in data/raw/MANIFEST.toml, or 'bibex raw check <path>' to ask whether one path is as recorded";
    match &rest[1..] {
        [verb] if verb == "bless" => Ok(RawVerb::Bless),
        [verb, path] if verb == "check" => Ok(RawVerb::Check(path.clone())),
        [verb] if verb == "check" => Err(CliError::bad_usage("'raw check' requires a <path> argument", USAGE, DO)),
        [] => Err(CliError::bad_usage("'raw' requires a verb", USAGE, DO)),
        other => Err(CliError::bad_usage(format!("unrecognized arguments for 'raw': {}", other.join(" ")), USAGE, DO)),
    }
}

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
            commands::node::run(&loaded.data, &loaded.graph, id_raw)
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
        "verify" => {
            let only = parse_verify_args(&rest)?;
            commands::verify::run(&data_dir, only.as_deref())
        }
        "raw" => match parse_raw_verb(&rest)? {
            RawVerb::Bless => commands::raw::bless(&data_dir),
            RawVerb::Check(path) => commands::raw::check(&data_dir, &path),
        },
        other => Err(CliError::bad_usage(
            format!("unrecognized subcommand '{other}'"),
            "'atlas' only knows verse, chapter, node, edges, find, kinds, verify, raw, tutorial, help",
            "run 'bibex help' for the full list",
        )),
    }
}

/// `tutorial`, `help` and a bare invocation are `bad_usage` here: a tutorial is prose by
/// nature, and the hint says so.
fn run_json(args: &[String]) -> Result<serde_json::Value, CliError> {
    let (data_dir, rest) = extract_data_dir(args)?;

    let Some(cmd) = rest.first() else {
        return Err(CliError::bad_usage(
            "a bare invocation has no --json output",
            "a bare invocation prints a prose help block, not a machine-shaped answer",
            "run 'bibex' or 'bibex help' without --json, or use --json with a real query command (verse/chapter/node/edges/find/kinds/verify)",
        ));
    };

    match cmd.as_str() {
        "help" => Err(CliError::bad_usage(
            "'help' has no --json output",
            "help is a prose command list, not a machine-shaped answer",
            "run 'bibex help' without --json, or use --json with a real query command (verse/chapter/node/edges/find/kinds/verify)",
        )),
        "tutorial" => Err(CliError::bad_usage(
            "'tutorial' has no --json output",
            "a tutorial is a guided prose walkthrough by nature, not a machine-shaped answer",
            "run 'bibex tutorial' without --json, or use --json with a real query command (verse/chapter/node/edges/find/kinds/verify)",
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
            commands::node::run_json(&loaded.data, &loaded.graph, id_raw)
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
        "verify" => {
            let only = parse_verify_args(&rest)?;
            commands::verify::run_json(&data_dir, only.as_deref())
        }
        "raw" => match parse_raw_verb(&rest)? {
            RawVerb::Bless => commands::raw::bless_json(&data_dir),
            RawVerb::Check(path) => commands::raw::check_json(&data_dir, &path),
        },
        other => Err(CliError::bad_usage(
            format!("unrecognized subcommand '{other}'"),
            "'atlas' only knows verse, chapter, node, edges, find, kinds, verify, raw, tutorial, help",
            "run 'bibex help' for the full list",
        )),
    }
}

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

    let mut i = 1;
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
