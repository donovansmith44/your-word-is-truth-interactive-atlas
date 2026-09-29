use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn contract_dir() -> PathBuf {
    repo_root().join("contracts").join("atlas-query-contract")
}

const SLASH_BEARING_WIRE_ID: &str = "CommentaryItem:kretzmann/0.1.0";

#[test]
fn a_wire_id_is_path_encoded_only_where_a_slash_would_split_the_route() {
    // Arrange
    let id = SLASH_BEARING_WIRE_ID;

    // Act
    let encoded = atlas_contract::aqc_export::path_encode(id);

    // Assert
    assert_eq!(encoded, "CommentaryItem:kretzmann%2F0.1.0");
}

#[test]
fn regenerated_features_match_the_committed_files() {
    let features_dir = contract_dir().join("features");

    let committed_focus = std::fs::read_to_string(features_dir.join("focus-query.feature")).expect("focus-query.feature must exist");
    assert_eq!(
        committed_focus,
        atlas_contract::aqc_export::focus_query_feature(),
        "focus-query.feature has drifted from what export_aqc_examples would regenerate -- run `cargo run -p atlas-contract --bin export_aqc_examples` from server/ and commit the result"
    );

    let committed_roundtrip = std::fs::read_to_string(features_dir.join("exploration-roundtrip.feature")).expect("exploration-roundtrip.feature must exist");
    assert_eq!(
        committed_roundtrip,
        atlas_contract::aqc_export::exploration_roundtrip_feature(),
        "exploration-roundtrip.feature has drifted from what export_aqc_examples would regenerate -- run `cargo run -p atlas-contract --bin export_aqc_examples` from server/ and commit the result"
    );
}

#[test]
fn every_seed_and_fixture_name_has_a_committed_file_and_vice_versa() {
    use atlas_contract::aqc_export::{FIXTURES, FOCUS_IDENTITY_EXTRA, SEEDS};

    let fixtures_dir = contract_dir().join("fixtures");

    let mut declared: BTreeSet<String> = BTreeSet::new();
    for (kind, _) in SEEDS {
        declared.insert(format!("focus-{}", kind.to_lowercase()));
    }
    for (name, _) in FIXTURES {
        declared.insert((*name).to_string());
    }
    for (_, name) in FOCUS_IDENTITY_EXTRA {
        declared.insert((*name).to_string());
    }

    for name in &declared {
        let path = fixtures_dir.join(format!("{name}.json"));
        assert!(path.exists(), "SEEDS/FIXTURES declares fixture '{name}' but {} does not exist -- run the exporter", path.display());
    }

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
    use atlas_contract::aqc_export::{FOCUS_IDENTITY_EXTRA, SEEDS};

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

#[test]
fn version_file_and_schema_version_agree_with_the_compiled_server_constants() {
    let version_path = contract_dir().join("VERSION");
    let version = std::fs::read_to_string(&version_path).expect("VERSION must exist");
    let version = version.trim();

    assert_eq!(version, atlas_contract::meta::MIN_SUPPORTED_VERSION, "contracts/atlas-query-contract/VERSION has drifted from meta::MIN_SUPPORTED_VERSION");
    assert_eq!(version, atlas_contract::meta::MAX_SUPPORTED_VERSION, "contracts/atlas-query-contract/VERSION has drifted from meta::MAX_SUPPORTED_VERSION");

    let schema_path = contract_dir().join("aqc.schema.json");
    let schema: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&schema_path).expect("aqc.schema.json must exist")).expect("aqc.schema.json must be valid JSON");
    let schema_version = schema["version"].as_str().expect("aqc.schema.json must have a top-level 'version' string");
    assert_eq!(schema_version, version, "aqc.schema.json's own 'version' has drifted from VERSION");
}

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
                    saw_header = true;
                } else {
                    total += 1;
                }
            }
        }
    }
    total
}

const EXPECTED_SCENARIO_COUNT: usize = 47;

#[test]
fn declared_scenario_count_matches_the_pinned_corpus_size() {
    assert_eq!(
        count_scenarios_in_feature_files(),
        EXPECTED_SCENARIO_COUNT,
        "the corpus's own scenario count has changed -- update EXPECTED_SCENARIO_COUNT here AND client.ContractTests's own CorpusCountTests AND server/Cargo.toml's STANDING COUNTING PROCEDURE block, in the same commit"
    );
}
