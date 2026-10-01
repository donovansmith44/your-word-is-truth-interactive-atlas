use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;

use atlas_contract::aqc_export::{FIXTURES, FOCUS_IDENTITY_EXTRA, SEEDS};

const EXPORTER: &str = env!("CARGO_BIN_EXE_export_aqc_examples");
const CLEAN_EXIT: Option<i32> = Some(0);
const MISUSE_EXIT: Option<i32> = Some(2);
const USAGE_LINE: &str = "usage: export_aqc_examples [CONTRACT_DIR]\n";
const EXPORTED_DIRECTORIES: [&str; 2] = ["features", "fixtures"];

#[test]
fn the_exporter_regenerates_the_published_corpus_byte_for_byte_and_says_what_it_wrote() {
    // Arrange
    let scratch = ScratchDirectory::new("regenerated_aqc_corpus");
    let expected_report = format!(
        "export_aqc_examples: verified {} seeds against the real committed graph; wrote focus-query.feature + exploration-roundtrip.feature + {} fixture files + index.json ({} identity entries)\n",
        SEEDS.len(),
        SEEDS.len() + FIXTURES.len(),
        identity_entry_count()
    );

    // Act
    let run = Command::new(EXPORTER).arg(&scratch.0).output().expect("the export_aqc_examples binary must run");

    // Assert
    let exported = exported_under(&scratch.0);
    assert_eq!(
        (run.status.code(), String::from_utf8_lossy(&run.stdout).replace("\r\n", "\n"), exported == committed_counterparts(&exported)),
        (CLEAN_EXIT, expected_report, true)
    );
}

#[test]
fn the_exporter_given_more_than_one_directory_refuses_with_the_usage_line_rather_than_writing() {
    // Arrange
    let mut exporter = Command::new(EXPORTER);
    exporter.args(["one", "two"]);

    // Act
    let run = exporter.output().expect("the export_aqc_examples binary must run");

    // Assert
    assert_eq!(
        (run.status.code(), String::from_utf8_lossy(&run.stdout).into_owned(), String::from_utf8_lossy(&run.stderr).into_owned()),
        (MISUSE_EXIT, String::new(), USAGE_LINE.to_string())
    );
}

fn contract_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../contracts/atlas-query-contract")
}

fn exported_under(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    let mut out = BTreeMap::new();
    for dir in EXPORTED_DIRECTORIES {
        let dir = root.join(dir);
        for entry in std::fs::read_dir(&dir).unwrap_or_else(|e| panic!("reading {}: {e}", dir.display())) {
            let path = entry.expect("a readable directory entry").path();
            let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("reading {}: {e}", path.display()));
            out.insert(path.strip_prefix(root).expect("a file under the directory read").to_path_buf(), bytes);
        }
    }
    out
}

fn committed_counterparts(exported: &BTreeMap<PathBuf, Vec<u8>>) -> BTreeMap<PathBuf, Vec<u8>> {
    exported
        .keys()
        .map(|relative| {
            let path = contract_dir().join(relative);
            (relative.clone(), std::fs::read(&path).unwrap_or_else(|e| panic!("reading {}: {e}", path.display())))
        })
        .collect()
}

fn identity_entry_count() -> usize {
    let mut ids: BTreeSet<&str> = SEEDS.iter().map(|(_, wire_id)| *wire_id).collect();
    ids.extend(FOCUS_IDENTITY_EXTRA.iter().map(|(wire_id, _)| *wire_id));
    ids.len()
}

struct ScratchDirectory(PathBuf);

impl ScratchDirectory {
    fn new(name: &str) -> ScratchDirectory {
        let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
        if path.exists() {
            std::fs::remove_dir_all(&path).unwrap_or_else(|e| panic!("emptying {}: {e}", path.display()));
        }
        std::fs::create_dir_all(&path).unwrap_or_else(|e| panic!("making {}: {e}", path.display()));
        ScratchDirectory(path)
    }
}

impl Drop for ScratchDirectory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
