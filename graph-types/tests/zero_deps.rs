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

/// The non-blank, non-comment lines belonging to `[table]`. `None` if the
/// table is absent in every form.
///
/// FIX ROUND 1 (review I3-2): a header matched by EXACT string missed the
/// dotted spelling. TOML lets one dependency be written either way --
///
/// ```toml
/// [dependencies]
/// serde = "1"
/// ```
/// ```toml
/// [dependencies.serde]
/// version = "1"
/// ```
///
/// -- and the second form never matched `[dependencies]`, so the
/// zero-dependency law passed with a dependency sitting in the manifest:
/// the one thing it exists to prevent. A header now belongs to the table
/// when it IS `[table]` or STARTS `[table.`, both spellings accumulate
/// into one entry list (a manifest may use both), and a dotted header
/// counts as an entry ITSELF -- `[dependencies.serde]` declares a
/// dependency whether or not a single line follows it.
fn table_entries(toml: &str, table: &str) -> Option<Vec<String>> {
    let exact = format!("[{table}]");
    let dotted = format!("[{table}.");
    let mut found = false;
    let mut inside = false;
    let mut out = Vec::new();
    for line in toml.lines() {
        let t = line.trim();
        if t.starts_with('[') {
            let is_sub = t.starts_with(&dotted) && t.ends_with(']');
            inside = t == exact || is_sub;
            if inside {
                found = true;
                if is_sub {
                    out.push(t.to_string());
                }
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

/// The negative case the exact-string parser used to miss entirely: a
/// dependency written as a DOTTED sub-table. If this test ever passes
/// vacuously the law above is decorative, so it asserts the parser SEES
/// the dependency rather than asserting the manifest is clean.
#[test]
fn a_dotted_sub_table_cannot_hide_a_dependency() {
    let manifest = "[package]\nname = \"x\"\n\n[dependencies.serde]\nversion = \"1\"\n\n[features]\ncanon-ids = []\n";

    let deps = table_entries(manifest, "dependencies")
        .expect("`[dependencies.serde]` IS a dependencies table, dotted spelling or not");
    assert!(
        !deps.is_empty() && deps.iter().any(|e| e.contains("serde")),
        "a dotted dependency must be reported as an entry, got {deps:?}"
    );

    // The same for the other two forbidden tables...
    for (table, text) in [
        ("dev-dependencies", "[dev-dependencies.proptest]\nversion = \"1\"\n"),
        ("build-dependencies", "[build-dependencies.cc]\nversion = \"1\"\n"),
    ] {
        let found = table_entries(text, table).unwrap_or_else(|| panic!("[{table}.…] must be attributed to {table}"));
        assert!(!found.is_empty(), "[{table}.…] must count as an entry, got {found:?}");
    }

    // ...and a bare dotted header with NOTHING under it still declares a
    // dependency, so the header alone must count.
    let bare = table_entries("[dependencies.serde]\n", "dependencies").expect("attributed");
    assert_eq!(bare, vec!["[dependencies.serde]".to_string()]);

    // Neighbouring tables are unaffected: the walk does not leak.
    assert_eq!(
        table_entries(manifest, "features"),
        Some(vec!["canon-ids = []".to_string()])
    );
    assert_eq!(table_entries(manifest, "dev-dependencies"), None);
}
