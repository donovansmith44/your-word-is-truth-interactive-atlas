//! Batch S requirement 3 fail-loud drift test: `data/curated/sources.toml`
//! must reconcile 1:1 against LICENSES.md's own "## Per-source table" --
//! this is the CI-visible enforcement of the Sources page's own
//! single-source-of-truth contract. `gen_sources`'s own binary runs the
//! same check at generation time; this test guarantees the check can
//! never silently go stale between generations, since `cargo test
//! --workspace` already runs it every time, with nothing extra to
//! remember to invoke.

use std::fs;
use std::path::Path;

fn repo_root_file(rel: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("reading {}: {e}", path.display()))
}

#[test]
fn sources_toml_reconciles_1to1_against_licenses_md_per_source_table() {
    let toml_input = repo_root_file("data/curated/sources.toml");
    let doc = atlas_etl::sources::parse_sources(&toml_input).expect("data/curated/sources.toml must parse");
    atlas_etl::sources::validate_structure(&doc).expect("data/curated/sources.toml structural validation");

    let licenses_md = repo_root_file("LICENSES.md");
    atlas_etl::sources::validate_against_licenses(&doc, &licenses_md).expect(
        "data/curated/sources.toml must reconcile 1:1 against LICENSES.md's per-source table \
         (batch-s-brief.md requirement 3, fail-loud drift) -- see the panic message above for \
         exactly which row/entry is unmatched",
    );
}

#[test]
fn per_source_table_has_the_expected_row_count() {
    // Belt-and-suspenders: batch-s-brief.md's own finalization block names
    // 18 sources as of BASE (dcb7278) -- an independent, hardcoded count
    // (not derived from sources.toml itself) so a bug that accidentally
    // made both sides drift together in the SAME wrong direction still
    // gets caught. This assertion is expected to grow over time as new
    // rows are added to LICENSES.md; a failure here is a prompt to update
    // BOTH this number and sources.toml together, never silently one.
    let toml_input = repo_root_file("data/curated/sources.toml");
    let doc = atlas_etl::sources::parse_sources(&toml_input).expect("data/curated/sources.toml must parse");
    assert_eq!(
        doc.sources.len(),
        21,
        "data/curated/sources.toml has {} entries, expected 21 (batch-s-brief.md's own finalization \
         count of 18 at BASE dcb7278, + 3 at LEX-1: STEPBible, MACULA, Strong's) -- if a real source \
         was intentionally added/removed, update this expected count in the same commit",
        doc.sources.len()
    );
}

#[test]
fn compiled_sources_json_matches_a_fresh_generation_from_curated_toml() {
    // Guards against `gen_sources` having been run against a stale
    // sources.toml (or not re-run after an edit at all) -- the compiled
    // artifact must always equal a fresh parse of the curated source.
    let toml_input = repo_root_file("data/curated/sources.toml");
    let doc = atlas_etl::sources::parse_sources(&toml_input).expect("data/curated/sources.toml must parse");

    let compiled = repo_root_file("data/compiled/sources.json");
    let on_disk: atlas_core::sources::SourcesDocument =
        serde_json::from_str(&compiled).expect("data/compiled/sources.json must parse");

    assert_eq!(
        doc, on_disk,
        "data/compiled/sources.json is stale relative to data/curated/sources.toml -- \
         re-run `cargo run -p atlas-etl --bin gen_sources` from server/ and commit the result"
    );
}

/// Batch PROV-1: the provenance join table's own structural checks, run
/// against the REAL committed `sources.toml` -- unique ids, every `source`
/// names a real `[[source]]`, every `confidence` is in the closed
/// vocabulary. (This is already covered transitively by
/// `sources_toml_reconciles_1to1_against_licenses_md_per_source_table`
/// above, which calls the same `validate_structure`; this test exists to
/// name the failure when it is a PROVENANCE row and not a source row, so a
/// reader of a red build is not sent to LICENSES.md for a problem that has
/// nothing to do with it.)
#[test]
fn the_provenance_join_table_is_structurally_sound() {
    let toml_input = repo_root_file("data/curated/sources.toml");
    let doc = atlas_etl::sources::parse_sources(&toml_input).expect("data/curated/sources.toml must parse");
    assert!(!doc.provenances.is_empty(), "the [[provenance]] join table must not be empty -- every piece of data needs a source (owner order 1)");
    atlas_etl::sources::validate_structure(&doc).expect("the [[provenance]] rows must name real sources and real confidences");
}

/// Batch PROV-1: the two ends of the pipeline agree about the NEW table
/// specifically. `compiled_sources_json_matches_a_fresh_generation_from_
/// curated_toml` above already asserts whole-document equality, which
/// covers this -- but a `#[serde(default)]` field that silently
/// deserializes to an empty Vec on BOTH sides would satisfy that equality
/// while carrying nothing at all to the browser, so this asserts the
/// compiled artifact is non-empty in its own right.
#[test]
fn the_compiled_sources_json_actually_carries_the_provenance_table() {
    let compiled = repo_root_file("data/compiled/sources.json");
    let on_disk: atlas_core::sources::SourcesDocument =
        serde_json::from_str(&compiled).expect("data/compiled/sources.json must parse");
    assert!(
        !on_disk.provenances.is_empty(),
        "data/compiled/sources.json carries no provenance rows -- re-run `cargo run -p atlas-etl --bin gen_sources` from server/ and commit the result"
    );
}

/// Batch PROV-1: a red case for each of the three structural rules, so the
/// checks above are proven to actually FAIL on bad input rather than merely
/// passing on good input (the red-green discipline this repo applies to
/// every law it adds).
#[test]
fn a_malformed_provenance_row_fails_validation_loudly() {
    const BASE: &str = r#"
[[category]]
id = "c"
label = "C"

[[source]]
id = "s"
category = "c"
title = "S"
what_it_is = "x"
what_we_built = "y"
license = "z"
licenses_row_key = "k"
"#;

    let check = |extra: &str, needle: &str, why: &str| {
        let doc = atlas_etl::sources::parse_sources(&format!("{BASE}{extra}")).expect("the fixture must parse");
        let err = atlas_etl::sources::validate_structure(&doc).expect_err(why);
        assert!(err.to_string().contains(needle), "expected an error mentioning '{needle}', got: {err}");
    };

    check(
        r#"
[[provenance]]
id = "p"
source = "nope"
confidence = "Imported"
"#,
        "undeclared source",
        "a provenance row naming no source must fail -- it would resolve to nothing at the UI, the fail-loud law's own silent-blank failure mode",
    );

    check(
        r#"
[[provenance]]
id = "p"
source = "s"
confidence = "Probably"
"#,
        "confidence",
        "an off-vocabulary confidence must fail -- it would render a label no ingest::Confidence variant backs",
    );

    check(
        r#"
[[provenance]]
id = "p"
source = "s"
confidence = "Imported"

[[provenance]]
id = "p"
source = "s"
confidence = "Curated"
"#,
        "duplicate provenance id",
        "a duplicate provenance id must fail -- resolution would be ambiguous",
    );

    check(
        r#"
[[provenance]]
id = "p"
source = "s"
confidence = "Imported"
locator = "   "
"#,
        "empty locator",
        "a blank locator must fail -- omit the key instead of rendering 'Locator: '",
    );
}
