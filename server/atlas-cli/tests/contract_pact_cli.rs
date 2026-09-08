//! Batch CDC-1: the CLI's own fragment of the recorded pact.
//!
//! bibex is a transport over the graph, so it is a contract surface, and
//! `contracts/atlas-graph-contract/transport/cli.feature` asks it the same
//! questions the HTTP suite asks the Router -- deliberately without a
//! fixture, so the only way those scenarios can pass is for the two
//! carriers to actually agree.
//!
//! # Why this lives here and not next to the other recorder
//!
//! `env!("CARGO_BIN_EXE_bibex")` is defined only for the tests of the
//! package that declares the binary. Reaching across the crate boundary
//! would mean `cargo run`-ing bibex from atlas-server's test, which builds
//! a second time and depends on the workspace layout. One recorder per
//! crate that OWNS a transport is the smaller rule, and the runner merges
//! every `*.json` in `contracts/pacts/`.
//!
//! # What it does NOT duplicate
//!
//! It does not re-derive which bibex invocations the corpus asks for.
//! Deciding what a step line means is one rule and it lives in ONE place
//! (`atlas-server/tests/contract_pact.rs::request_key`), which publishes
//! the result as `cli_keys` in the committed HTTP pact. This recorder
//! reads that list. Two recorders parsing Gherkin two ways is exactly the
//! kind of second, weaker path that eventually disagrees with itself.
//!
//! Consequence, stated rather than discovered: adding a bibex step to a
//! feature file needs the HTTP recorder re-run first (to refresh
//! `cli_keys`), then this one. If you forget, nothing is silently missed
//! -- the runner fails with "no pact entry for 'bibex ...'", naming it.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{json, Map, Value};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn compiled_data_dir() -> PathBuf {
    repo_root().join("data/compiled")
}

fn http_pact_path() -> PathBuf {
    repo_root().join("contracts/pacts/http.json")
}

fn cli_pact_path() -> PathBuf {
    repo_root().join("contracts/pacts/cli.json")
}

/// The bibex invocations the corpus asks for, as published by the HTTP
/// recorder. Absent or empty is a hard failure, not an empty run: "no
/// bibex keys" and "the HTTP pact has not been generated yet" look
/// identical from here, and quietly writing an empty CLI pact would turn
/// the second case into a green test and a broken gate.
fn cli_keys() -> Vec<String> {
    let path = http_pact_path();
    let raw = std::fs::read_to_string(&path).unwrap_or_else(|_| {
        panic!(
            "no HTTP pact at {} -- it publishes the bibex request keys this recorder works from.\n  Generate it first with:\n    ATLAS_BLESS_PACT=1 cargo test -p atlas-server --test contract_pact",
            path.display()
        )
    });
    let v: Value = serde_json::from_str(&raw).expect("the committed HTTP pact must be JSON");
    let keys: Vec<String> = v["cli_keys"]
        .as_array()
        .unwrap_or_else(|| panic!("{} carries no cli_keys array", path.display()))
        .iter()
        .map(|k| k.as_str().expect("every cli key is a string").to_string())
        .collect();
    assert!(
        !keys.is_empty(),
        "the HTTP pact publishes an EMPTY cli_keys list, but transport/cli.feature exists -- re-run the HTTP recorder"
    );
    keys
}

/// Run bibex exactly as a consumer would, against the real committed
/// graph, and capture its `--json` answer.
///
/// `--json` is bibex's contract surface (its own Cargo.toml: "BIBEX-1
/// (--json flag, contract-first)"). Its human-readable stdout is pinned as
/// a transcript by `tests/cli.rs`, which is the right tool for prose; this
/// records the machine surface a second system would bind to.
fn record_bibex(args: &str) -> Value {
    let dd = compiled_data_dir();
    let dd_str = dd.to_str().expect("the data dir path must be valid UTF-8");
    let mut argv: Vec<String> = vec!["--data-dir".into(), dd_str.into(), "--json".into()];
    argv.extend(args.split_whitespace().map(str::to_string));

    let out = Command::new(env!("CARGO_BIN_EXE_bibex"))
        .args(&argv)
        .output()
        .expect("bibex must run");

    let stdout = String::from_utf8(out.stdout).expect("bibex stdout must be valid UTF-8");
    assert!(
        out.status.success(),
        "`bibex --json {args}` failed ({});\n  stdout: {stdout}\n  stderr: {}",
        out.status,
        String::from_utf8_lossy(&out.stderr)
    );
    let body: Value = serde_json::from_str(&stdout).unwrap_or_else(|e| {
        panic!("`bibex --json {args}` did not print one JSON value ({e}) -- the --json happy path promises exactly that\n  stdout: {stdout}")
    });
    json!({ "status": 200, "body": body })
}

fn build_pact() -> Value {
    let mut entries = Map::new();
    for key in cli_keys() {
        let args = key.strip_prefix("bibex ").expect("every cli key starts with 'bibex '");
        entries.insert(key.clone(), record_bibex(args));
    }
    json!({ "entries": Value::Object(entries) })
}

fn render(v: &Value) -> String {
    let mut s = serde_json::to_string_pretty(v).expect("a pact must serialize");
    s.push('\n');
    s
}

/// Same law as the HTTP recorder's: regenerate from the real binary
/// against the real graph, and fail on any difference from the committed
/// copy. `ATLAS_BLESS_PACT=1` re-records -- a re-record switch, not a gate
/// escape hatch: it changes what the evidence says, never whether the gate
/// runs.
#[test]
fn the_recorded_cli_pact_still_matches_the_real_binary() {
    let rendered = render(&build_pact());
    let path = cli_pact_path();

    if std::env::var("ATLAS_BLESS_PACT").as_deref() == Ok("1") {
        std::fs::create_dir_all(path.parent().expect("the pact has a parent directory"))
            .expect("the pact directory must be creatable");
        std::fs::write(&path, &rendered).expect("the pact must be writable");
        eprintln!("ATLAS_BLESS_PACT=1: re-recorded {} ({} bytes)", path.display(), rendered.len());
        return;
    }

    let committed = std::fs::read_to_string(&path).unwrap_or_else(|_| {
        panic!(
            "no recorded CLI pact at {}.\n  Generate it with:\n    ATLAS_BLESS_PACT=1 cargo test -p atlas-cli --test contract_pact_cli",
            path.display()
        )
    });

    assert_eq!(
        committed, rendered,
        "bibex's --json answers drifted from the recorded pact.\n  If this change is DELIBERATE it is a contract change: bump the suite VERSION, add a CHANGELOG entry, and re-record with ATLAS_BLESS_PACT=1."
    );
}
