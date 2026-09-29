use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;

use atlas_contract::aqc_export::{FIXTURES, FOCUS_IDENTITY_EXTRA, SEEDS};

const EXPORTER: &str = env!("CARGO_BIN_EXE_export_aqc_examples");
const CLEAN_EXIT: Option<i32> = Some(0);

fn contract_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../contracts/atlas-query-contract")
}

/// Every file the exporter writes, by path, so "the exporter changed nothing" is one
/// comparison rather than a file-by-file walk.
fn published_corpus() -> BTreeMap<PathBuf, Vec<u8>> {
    let mut out = BTreeMap::new();
    for dir in ["features", "fixtures"] {
        let dir = contract_dir().join(dir);
        for entry in std::fs::read_dir(&dir).unwrap_or_else(|e| panic!("reading {}: {e}", dir.display())) {
            let path = entry.expect("a readable directory entry").path();
            let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("reading {}: {e}", path.display()));
            out.insert(path, bytes);
        }
    }
    out
}

fn identity_entry_count() -> usize {
    let mut ids: BTreeSet<&str> = SEEDS.iter().map(|(_, wire_id)| *wire_id).collect();
    ids.extend(FOCUS_IDENTITY_EXTRA.iter().map(|(wire_id, _)| *wire_id));
    ids.len()
}

#[test]
fn the_exporter_regenerates_the_published_corpus_byte_for_byte_and_says_what_it_wrote() {
    // Arrange
    let before = published_corpus();
    let expected_report = format!(
        "export_aqc_examples: verified {} seeds against the real committed graph; wrote focus-query.feature + exploration-roundtrip.feature + {} fixture files + index.json ({} identity entries)\n",
        SEEDS.len(),
        SEEDS.len() + FIXTURES.len(),
        identity_entry_count()
    );

    // Act
    let run = Command::new(EXPORTER).output().expect("the export_aqc_examples binary must run");

    // Assert
    assert_eq!(
        (run.status.code(), String::from_utf8_lossy(&run.stdout).replace("\r\n", "\n"), published_corpus() == before),
        (CLEAN_EXIT, expected_report, true)
    );
}
