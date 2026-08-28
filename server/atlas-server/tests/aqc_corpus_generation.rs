//! Batch AQC-1 fix round 1 (Q-6, controller ruling): "generated-output-is-
//! committed + a test that regenerates and diffs," the pattern this repo
//! already uses elsewhere. Two standing checks, neither needing a live
//! server or the real graph (both are PURE, fast):
//!
//! 1. The two `Scenario Outline`-bearing `.feature` files this repo
//!    commits are byte-identical to what `atlas_server::aqc_export`'s own
//!    generation functions produce RIGHT NOW -- catches drift the moment
//!    someone hand-edits a committed feature file, or the SEEDS list
//!    changes without re-running the exporter.
//! 2. Every `SEEDS`/`FOCUS_IDENTITY_EXTRA`/`FIXTURES` name has a committed
//!    fixture file, AND vice versa (every committed fixture corresponds to
//!    a name one of those three sources declares) -- the exact class of
//!    gap that let `focus-traversal-target.json` go briefly unindexed
//!    during this batch's own C# harness work (batch report, "gap found
//!    and closed") would have failed THIS test automatically.
//!
//! Plus the five-hand-kept-"0.1.0"-copies cross-check (Q-6's other half):
//! `VERSION`'s trimmed contents, `aqc.schema.json`'s own top-level
//! `"version"`, and `contract::{MIN,MAX}_SUPPORTED_VERSION` must all agree.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn contract_dir() -> PathBuf {
    repo_root().join("contracts").join("atlas-query-contract")
}

#[test]
fn regenerated_features_match_the_committed_files() {
    let features_dir = contract_dir().join("features");

    let committed_focus = std::fs::read_to_string(features_dir.join("focus-query.feature")).expect("focus-query.feature must exist");
    assert_eq!(
        committed_focus,
        atlas_server::aqc_export::focus_query_feature(),
        "focus-query.feature has drifted from what export_aqc_examples would regenerate -- run `cargo run -p atlas-server --bin export_aqc_examples` from server/ and commit the result"
    );

    let committed_roundtrip = std::fs::read_to_string(features_dir.join("exploration-roundtrip.feature")).expect("exploration-roundtrip.feature must exist");
    assert_eq!(
        committed_roundtrip,
        atlas_server::aqc_export::exploration_roundtrip_feature(),
        "exploration-roundtrip.feature has drifted from what export_aqc_examples would regenerate -- run `cargo run -p atlas-server --bin export_aqc_examples` from server/ and commit the result"
    );
}

#[test]
fn every_seed_and_fixture_name_has_a_committed_file_and_vice_versa() {
    use atlas_server::aqc_export::{FIXTURES, FOCUS_IDENTITY_EXTRA, SEEDS};

    let fixtures_dir = contract_dir().join("fixtures");

    // Forward: every name the generator declares must have a file.
    let mut declared: BTreeSet<String> = BTreeSet::new();
    for (kind, _) in SEEDS {
        declared.insert(format!("focus-{}", kind.to_lowercase()));
    }
    for (name, _) in FIXTURES {
        declared.insert((*name).to_string());
    }
    // FOCUS_IDENTITY_EXTRA names are already covered by FIXTURES (both
    // list "focus-traversal-target"); included here too so this test
    // does not silently rely on that overlap staying true.
    for (_, name) in FOCUS_IDENTITY_EXTRA {
        declared.insert((*name).to_string());
    }

    for name in &declared {
        let path = fixtures_dir.join(format!("{name}.json"));
        assert!(path.exists(), "SEEDS/FIXTURES declares fixture '{name}' but {} does not exist -- run the exporter", path.display());
    }

    // Backward: every committed fixture file (except index.json, which is
    // not a query-response fixture) must correspond to a declared name --
    // an orphaned fixture is exactly as much drift as a missing one.
    let mut committed: BTreeSet<String> = BTreeSet::new();
    for entry in std::fs::read_dir(&fixtures_dir).expect("fixtures dir must exist") {
        let entry = entry.unwrap();
        let file_name = entry.file_name().to_string_lossy().to_string();
        if let Some(stem) = file_name.strip_suffix(".json") {
            if stem != "index" {
                committed.insert(stem.to_string());
            }
        }
    }

    let orphaned: Vec<&String> = committed.difference(&declared).collect();
    assert!(orphaned.is_empty(), "committed fixture(s) {orphaned:?} have no corresponding SEEDS/FIXTURES/FOCUS_IDENTITY_EXTRA entry -- dead fixture, or the exporter's own tables are missing an entry");

    let missing: Vec<&String> = declared.difference(&committed).collect();
    assert!(missing.is_empty(), "SEEDS/FIXTURES/FOCUS_IDENTITY_EXTRA declares {missing:?} but no committed fixture file exists -- run the exporter");
}

#[test]
fn index_json_matches_the_identity_declared_in_seeds_and_focus_identity_extra() {
    use atlas_server::aqc_export::{FOCUS_IDENTITY_EXTRA, SEEDS};

    let index_path = contract_dir().join("fixtures").join("index.json");
    let index: std::collections::BTreeMap<String, String> = serde_json::from_str(&std::fs::read_to_string(&index_path).expect("index.json must exist")).expect("index.json must be valid JSON");

    let mut expected: std::collections::BTreeMap<String, String> = std::collections::BTreeMap::new();
    for (kind, wire_id) in SEEDS {
        expected.insert((*wire_id).to_string(), format!("focus-{}", kind.to_lowercase()));
    }
    for (wire_id, name) in FOCUS_IDENTITY_EXTRA {
        expected.insert((*wire_id).to_string(), (*name).to_string());
    }

    assert_eq!(index, expected, "index.json has drifted from SEEDS/FOCUS_IDENTITY_EXTRA -- run the exporter and commit the result");
}

/// Q-6 fix: the five hand-kept "0.1.0" copies, cross-checked. `VERSION`'s
/// trimmed contents and `aqc.schema.json`'s own top-level `"version"` must
/// both agree with `contract::{MIN,MAX}_SUPPORTED_VERSION` -- the compiled
/// server's own constants, the one copy that genuinely cannot be read from
/// a file at compile time (a real server binary, not a test). The
/// remaining two copies (`AqcContract.cs::ClientVersion`, `aqc_cucumber
/// .rs`'s own `harness_client_version()`) are single-sourced from this
/// SAME `VERSION` file instead of hand-kept -- see that function's own doc
/// comment and `client.Tests/AqcContractTests.cs::
/// ClientVersionAgreesWithTheVersionFile`.
#[test]
fn version_file_and_schema_version_agree_with_the_compiled_server_constants() {
    let version_path = contract_dir().join("VERSION");
    let version = std::fs::read_to_string(&version_path).expect("VERSION must exist");
    let version = version.trim();

    assert_eq!(version, atlas_server::contract::MIN_SUPPORTED_VERSION, "contracts/atlas-query-contract/VERSION has drifted from contract::MIN_SUPPORTED_VERSION");
    assert_eq!(version, atlas_server::contract::MAX_SUPPORTED_VERSION, "contracts/atlas-query-contract/VERSION has drifted from contract::MAX_SUPPORTED_VERSION");

    let schema_path = contract_dir().join("aqc.schema.json");
    let schema: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&schema_path).expect("aqc.schema.json must exist")).expect("aqc.schema.json must be valid JSON");
    let schema_version = schema["version"].as_str().expect("aqc.schema.json must have a top-level 'version' string");
    assert_eq!(schema_version, version, "aqc.schema.json's own 'version' has drifted from VERSION");
}

/// THE COUNTING LAW (Batch AQC-1 fix round 1, controller ruling): "the
/// dotnet line carries the parity check: client.Tests N/N |
/// client.ContractTests X/X -- where client.ContractTests's count MUST
/// EQUAL the AQC line's X ... any divergence means a feature file is being
/// executed by one side and not the other."
///
/// Counts every scenario textually across all six committed `.feature`
/// files (a `Scenario:` line is 1; a `Scenario Outline:`'s own count is
/// its `Examples:` table's DATA rows, header excluded) -- pure text
/// parsing, no Gherkin-library dependency, so this stays independent of
/// whatever `cucumber`'s own internal parser does. `client.ContractTests`'s
/// own `CorpusCountTests` runs the SAME textual count in C#, independently,
/// and additionally cross-checks it against Reqnroll's own runtime-
/// discovered test count via reflection -- the two tests TOGETHER are what
/// makes "both sides execute the same files" (spec §3) a real, standing
/// assertion rather than a one-time observation: if a scenario is ever
/// added/removed from the corpus without both counts being updated in the
/// SAME commit, one of the two tests fails.
fn count_scenarios_in_feature_files() -> usize {
    let features_dir = contract_dir().join("features");
    let mut total = 0usize;
    for entry in std::fs::read_dir(&features_dir).expect("features dir must exist") {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("feature") {
            continue;
        }
        let text = std::fs::read_to_string(&path).unwrap();
        let mut in_examples = false;
        let mut saw_header = false;
        for raw_line in text.lines() {
            let line = raw_line.trim();
            if line.starts_with("Scenario Outline:") {
                in_examples = false;
                saw_header = false;
            } else if line.starts_with("Scenario:") {
                total += 1;
                in_examples = false;
                saw_header = false;
            } else if line.starts_with("Examples:") {
                in_examples = true;
                saw_header = false;
            } else if in_examples && line.starts_with('|') {
                if !saw_header {
                    saw_header = true; // the header row, not a data row
                } else {
                    total += 1;
                }
            }
        }
    }
    total
}

/// The current, disclosed truth this file and `CorpusCountTests.cs` both
/// pin -- bump in the SAME commit as any corpus scenario add/remove
/// (recorded here per the counting law's own ruling 4: "add this to
/// server/Cargo.toml's STANDING COUNTING PROCEDURE comment block").
const EXPECTED_SCENARIO_COUNT: usize = 44;

#[test]
fn declared_scenario_count_matches_the_pinned_corpus_size() {
    assert_eq!(
        count_scenarios_in_feature_files(),
        EXPECTED_SCENARIO_COUNT,
        "the corpus's own scenario count has changed -- update EXPECTED_SCENARIO_COUNT here AND client.ContractTests's own CorpusCountTests AND server/Cargo.toml's STANDING COUNTING PROCEDURE block, in the same commit"
    );
}
