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
    // 20 sources: 18 as of BASE (dcb7278), plus the two CATECH-V1 curated
    // files (reflection prompts, song bindings). An independent, hardcoded count
    // (not derived from sources.toml itself) so a bug that accidentally
    // made both sides drift together in the SAME wrong direction still
    // gets caught. This assertion is expected to grow over time as new
    // rows are added to LICENSES.md; a failure here is a prompt to update
    // BOTH this number and sources.toml together, never silently one.
    let toml_input = repo_root_file("data/curated/sources.toml");
    let doc = atlas_etl::sources::parse_sources(&toml_input).expect("data/curated/sources.toml must parse");
    assert_eq!(
        doc.sources.len(),
        20,
        "data/curated/sources.toml has {} entries, expected 20 (18 at BASE dcb7278 + 2 from CATECH-V1) -- if a real source was intentionally added/removed, update this \
         expected count in the same commit",
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
