//! DB-2a, amended by D9: "zero dependencies BY DEFAULT" is the LAW of this
//! crate now, not "zero dependencies, full stop".
//!
//! `graph-types` is the base point every other crate binds to, so a
//! dependency here must never reach a consumer that did not ask for it.
//! D9 lets `[dependencies]` carry `serde` and `utoipa` -- but only because
//! both are `optional = true`, gated by a feature nothing turns on by
//! default; `[dev-dependencies]` may carry `serde_json` (tests only, never
//! shipped to a consumer); `[build-dependencies]` still holds nothing.
//! `[features]` may hold exactly `canon-ids`, `serde` and `openapi`, and
//! never a `default` feature -- a `default` is the one thing that would
//! turn an optional dependency into a real one.
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

/// The exact `[dependencies]` entries the manifest declares today -- both
/// optional, so nothing reaches a consumer who does not opt in (D9). One
/// source of truth for the two tests below that both check against it.
fn expected_dependency_entries() -> Vec<String> {
    vec![
        "serde = { version = \"1\", optional = true, default-features = false }".to_string(),
        "utoipa = { version = \"6.0.0\", optional = true }".to_string(),
    ]
}

#[test]
fn every_dependency_is_optional_so_the_default_build_takes_none() {
    // Arrange
    let toml = std::fs::read_to_string(manifest_path()).expect("graph-types/Cargo.toml");
    // Act
    let entries = table_entries(&toml, "dependencies").expect("[dependencies] must exist");
    // Assert
    assert_eq!(
        entries,
        expected_dependency_entries(),
        "[dependencies] must hold exactly serde and utoipa, both `optional = true` -- \
         the default build takes neither"
    );
}

#[test]
fn dev_dependencies_hold_only_serde_json_for_tests() {
    // Arrange
    let toml = std::fs::read_to_string(manifest_path()).expect("graph-types/Cargo.toml");
    // Act
    let entries = table_entries(&toml, "dev-dependencies").expect("[dev-dependencies] must exist");
    // Assert
    assert_eq!(
        entries,
        vec!["serde_json = \"1\"".to_string()],
        "[dev-dependencies] must hold exactly serde_json -- test-only, never reaches a consumer"
    );
}

#[test]
fn build_dependencies_table_holds_no_entry() {
    // Arrange
    let toml = std::fs::read_to_string(manifest_path()).expect("graph-types/Cargo.toml");
    // Act
    let entries = table_entries(&toml, "build-dependencies");
    // Assert
    match entries {
        None => {} // absent is as good as empty
        Some(entries) => assert!(
            entries.is_empty(),
            "[build-dependencies] must be empty; found {entries:?}"
        ),
    }
}

#[test]
fn features_hold_exactly_canon_ids_serde_and_openapi_with_no_default() {
    // Arrange
    let toml = std::fs::read_to_string(manifest_path()).expect("graph-types/Cargo.toml");
    // Act
    let entries = table_entries(&toml, "features").expect("[features] must exist");
    // Assert
    assert_eq!(
        entries,
        vec![
            "canon-ids = []".to_string(),
            "serde = [\"dep:serde\"]".to_string(),
            "openapi = [\"dep:utoipa\"]".to_string(),
        ],
        "[features] must declare exactly canon-ids, serde and openapi, and nothing else"
    );
    assert!(
        !entries.iter().any(|e| e.starts_with("default")),
        "[features] must never declare a `default` feature -- that is what would turn \
         an optional dependency into a real one"
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

    // And the law itself still reads the REAL manifest correctly -- exactly
    // the two optional dependencies it declares (D9), not the empty table
    // the pre-D9 law expected.
    let toml = std::fs::read_to_string(manifest_path()).expect("graph-types/Cargo.toml");
    assert_eq!(table_entries(&toml, "dependencies"), Some(expected_dependency_entries()));
}

/// FINAL REVIEW item 12, second gap: a header with a trailing comment.
/// The crate's own `[features]` carries a comment block above it today;
/// one on the header LINE used to make the whole table invisible, which
/// would have turned `features_hold_exactly_canon_ids_serde_and_openapi_with_no_default`
/// into a test of nothing.
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
