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
///
/// FINAL REVIEW item 12: two more gaps, both in the header test.
///
/// 1. The PLATFORM-SCOPED spelling. `[target.'cfg(windows)'.dependencies]`
///    (and the unquoted `[target.cfg(unix).dependencies]`) declares a real
///    dependency, and neither `[dependencies]` nor `[dependencies.` sees
///    it. A header now belongs to the table when it ENDS `.{table}]` as
///    well -- a suffix rule, so `[dev-dependencies]` still cannot be read
///    as `dependencies` (it does not end `.dependencies]`) while every
///    `target`-scoped spelling can.
/// 2. A TRAILING COMMENT. `[features]  # the one switch` was not equal to
///    `[features]`, so the whole table vanished and its law passed on an
///    empty reading. A header's comment is stripped before comparing --
///    the `#` must be OUTSIDE any quoted cfg string, so a `#` inside
///    `'cfg(...)'` is left alone.
///
/// Crude in the safe direction, still: a header shape this cannot read
/// fails the test rather than passing it.
fn strip_header_comment(line: &str) -> &str {
    let bytes = line.as_bytes();
    let mut quote: Option<u8> = None;
    for (i, b) in bytes.iter().enumerate() {
        match quote {
            Some(q) if *b == q => quote = None,
            Some(_) => {}
            None if *b == b'\'' || *b == b'"' => quote = Some(*b),
            None if *b == b'#' => return line[..i].trim_end(),
            None => {}
        }
    }
    line
}

fn table_entries(toml: &str, table: &str) -> Option<Vec<String>> {
    let exact = format!("[{table}]");
    let dotted = format!("[{table}.");
    let suffix = format!(".{table}]");
    let mut found = false;
    let mut inside = false;
    let mut out = Vec::new();
    for line in toml.lines() {
        let t = line.trim();
        if t.starts_with('[') {
            let t = strip_header_comment(t);
            // `[table.…]` (dotted sub-table) or `[….table]` (target-scoped);
            // either way the header itself declares the table.
            let is_sub =
                (t.starts_with(&dotted) && t.ends_with(']')) || t.ends_with(&suffix);
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

/// FINAL REVIEW item 12, first gap: a PLATFORM-SCOPED dependency table.
/// Like the dotted case above, this asserts the parser SEES the
/// dependency -- a test that merely re-checked the clean manifest would
/// pass whether or not the rule exists.
#[test]
fn a_target_scoped_table_cannot_hide_a_dependency() {
    for (table, text) in [
        ("dependencies", "[target.'cfg(windows)'.dependencies]\nwinapi = \"0.3\"\n"),
        ("dependencies", "[target.cfg(unix).dependencies]\nlibc = \"0.2\"\n"),
        (
            "dev-dependencies",
            "[target.'cfg(target_os = \"linux\")'.dev-dependencies]\nproptest = \"1\"\n",
        ),
        ("build-dependencies", "[target.'cfg(windows)'.build-dependencies]\ncc = \"1\"\n"),
    ] {
        let found = table_entries(text, table)
            .unwrap_or_else(|| panic!("a target-scoped [{table}] must be attributed to {table}"));
        assert!(
            found.len() == 2,
            "the header ITSELF declares the table, and its one line is an entry: {found:?}"
        );
    }

    // The suffix rule is a SUFFIX, not a substring: `dev-dependencies` is
    // not `dependencies`, scoped or not.
    assert_eq!(
        table_entries("[target.'cfg(windows)'.dev-dependencies]\nproptest = \"1\"\n", "dependencies"),
        None,
        "`.dev-dependencies]` must not be read as `.dependencies]`"
    );

    // And the law itself still reads the REAL manifest as clean.
    let toml = std::fs::read_to_string(manifest_path()).expect("graph-types/Cargo.toml");
    assert_eq!(table_entries(&toml, "dependencies"), Some(Vec::new()));
}

/// FINAL REVIEW item 12, second gap: a header with a trailing comment.
/// The crate's own `[features]` carries a comment block above it today;
/// one on the header LINE used to make the whole table invisible, which
/// would have turned `features_holds_exactly_the_canon_ids_switch` into a
/// test of nothing.
#[test]
fn a_trailing_comment_on_a_header_does_not_hide_the_table() {
    let commented = "[dependencies]  # nothing lives here\nserde = \"1\"\n";
    assert_eq!(
        table_entries(commented, "dependencies"),
        Some(vec!["serde = \"1\"".to_string()]),
        "a commented header is still the header"
    );
    assert_eq!(
        table_entries("[features] # the one switch\ncanon-ids = []\n", "features"),
        Some(vec!["canon-ids = []".to_string()])
    );
    assert_eq!(
        table_entries("[dependencies.serde] # pinned\nversion = \"1\"\n", "dependencies"),
        Some(vec!["[dependencies.serde]".to_string(), "version = \"1\"".to_string()])
    );
    // A `#` INSIDE a quoted cfg is part of the header, not a comment: the
    // table is still attributed, and the stripper does not cut it short.
    let quoted = "[target.'cfg(feature = \"a#b\")'.dependencies]\nlibc = \"0.2\"\n";
    let found = table_entries(quoted, "dependencies").expect("attributed");
    assert_eq!(found.len(), 2, "got {found:?}");
    assert!(found[0].ends_with(".dependencies]"), "header kept whole: {:?}", found[0]);
}
