//! DB-2a: "zero dependencies" is a LAW of this crate, not a habit.
//!
//! `graph-types` is the base point every other crate binds to; a single
//! third-party crate here would put someone else's code inside every
//! content address the artifact carries. So the manifest itself is under
//! test: no `[dependencies]`, `[dev-dependencies]` or `[build-dependencies]`
//! table may hold ANY entry, and `[features]` may hold exactly one --
//! `canon-ids = []`, the feature this batch adds.
//!
//! The parser is hand-written std (a TOML crate here would be the very
//! dependency the test forbids). It is deliberately crude and deliberately
//! STRICT: a table is the lines from its header to the next header, and
//! every non-blank, non-comment line in it counts as an entry. Crude in
//! the safe direction -- a manifest shape it cannot read fails the test
//! rather than passing it.

use std::path::{Path, PathBuf};

fn manifest_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml")
}

/// The non-blank, non-comment lines belonging to `[table]` -- from its
/// header to the next header of any kind. `None` if the table is absent.
fn table_entries(toml: &str, table: &str) -> Option<Vec<String>> {
    let header = format!("[{table}]");
    let mut found = false;
    let mut inside = false;
    let mut out = Vec::new();
    for line in toml.lines() {
        let t = line.trim();
        if t.starts_with('[') {
            if inside {
                break; // the next header closes the table
            }
            if t == header {
                inside = true;
                found = true;
            }
            continue;
        }
        if inside && !t.is_empty() && !t.starts_with('#') {
            out.push(t.to_string());
        }
    }
    if found {
        Some(out)
    } else {
        None
    }
}

#[test]
fn no_dependency_table_holds_an_entry() {
    let toml = std::fs::read_to_string(manifest_path()).expect("graph-types/Cargo.toml");
    for table in ["dependencies", "dev-dependencies", "build-dependencies"] {
        match table_entries(&toml, table) {
            None => {} // absent is as good as empty
            Some(entries) => assert!(
                entries.is_empty(),
                "[{table}] must be empty; found {entries:?} -- graph-types is zero-dependency"
            ),
        }
    }
}

#[test]
fn features_holds_exactly_the_canon_ids_switch() {
    let toml = std::fs::read_to_string(manifest_path()).expect("graph-types/Cargo.toml");
    let entries = table_entries(&toml, "features").expect("[features] must exist");
    assert_eq!(
        entries,
        vec!["canon-ids = []".to_string()],
        "[features] must declare exactly `canon-ids = []` and nothing else"
    );
}

#[test]
fn the_parser_reads_the_manifest_shapes_we_write() {
    let sample = "[package]\nname = \"x\"\n\n[dependencies]\n\n[features]\n# a comment\ncanon-ids = []\n";
    assert_eq!(table_entries(sample, "dependencies"), Some(Vec::new()));
    assert_eq!(
        table_entries(sample, "features"),
        Some(vec!["canon-ids = []".to_string()])
    );
    assert_eq!(table_entries(sample, "dev-dependencies"), None);
    // A populated table is seen as populated.
    let dirty = "[dependencies]\nserde = \"1\"\n";
    assert_eq!(
        table_entries(dirty, "dependencies"),
        Some(vec!["serde = \"1\"".to_string()])
    );
    // A table that runs to EOF is still closed correctly.
    assert_eq!(table_entries("[features]\ncanon-ids = []\n", "features"), Some(vec!["canon-ids = []".to_string()]));
}
