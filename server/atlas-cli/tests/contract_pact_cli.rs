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

fn cli_keys() -> Vec<String> {
    let path = http_pact_path();
    let raw = std::fs::read_to_string(&path).unwrap_or_else(|_| {
        panic!(
            "no HTTP pact at {} -- it publishes the bibex request keys this recorder works from.\n  Generate it first with:\n    ATLAS_BLESS_PACT=1 cargo test -p atlas-contract --test contract_pact",
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

#[test]
fn the_recorded_cli_pact_still_matches_the_real_binary() {
    let rendered = render(&build_pact());
    let path = cli_pact_path();

    if std::env::var("ATLAS_BLESS_PACT").as_deref() == Ok("1") {
        std::fs::create_dir_all(path.parent().expect("the pact has a parent directory"))
            .expect("the pact directory must be creatable");
        std::fs::write(&path, &rendered).expect("the pact must be writable");
        panic!(
            "ATLAS_BLESS_PACT=1: RE-RECORDED {} ({} bytes) -- failing on purpose.\n  \
             Blessing rewrites the evidence, so it must never be able to report a passing gate.\n  \
             Review the diff, then re-run WITHOUT ATLAS_BLESS_PACT to verify against it.",
            path.display(),
            rendered.len()
        );
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
