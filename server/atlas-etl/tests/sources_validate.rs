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
    let toml_input = repo_root_file("data/curated/sources.toml");
    let doc = atlas_etl::sources::parse_sources(&toml_input).expect("data/curated/sources.toml must parse");
    assert_eq!(
        doc.sources.len(),
        20,
        "data/curated/sources.toml has {} entries, expected 20 (batch-s-brief.md's own finalization \
         count of 18 at BASE dcb7278, + 3 at LEX-1: STEPBible, MACULA, Strong's, - MACULA when its \
         ShareAlike domain codes were dropped) -- if a real source \
         was intentionally added/removed, update this expected count in the same commit",
        doc.sources.len()
    );
}

#[test]
fn compiled_sources_json_matches_a_fresh_generation_from_curated_toml() {
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

#[test]
fn the_provenance_join_table_is_structurally_sound() {
    let toml_input = repo_root_file("data/curated/sources.toml");
    let doc = atlas_etl::sources::parse_sources(&toml_input).expect("data/curated/sources.toml must parse");
    assert!(!doc.provenances.is_empty(), "the [[provenance]] join table must not be empty -- every piece of data needs a source (owner order 1)");
    atlas_etl::sources::validate_structure(&doc).expect("the [[provenance]] rows must name real sources and real confidences");
}

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

/// One category and one source, so a `[[provenance]]` row under it names something real.
const PROVENANCE_BASE: &str = r#"
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

const CONFIDENCE_REFUSAL: &str = "sources.toml: invalid TOML or does not match the [[category]]/[[source]]/[[provenance]] schema: TOML parse error at line 18, column 14
   |
18 | confidence = \"Probably\"
   |              ^^^^^^^^^^
unknown variant `Probably`, expected one of `CanonicalText`, `Curated`, `Imported`, `Derived`
";

#[test]
fn a_malformed_provenance_row_fails_validation_loudly() {
    let check = |extra: &str, needle: &str, why: &str| {
        let doc = atlas_etl::sources::parse_sources(&format!("{PROVENANCE_BASE}{extra}")).expect("the fixture must parse");
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

#[test]
fn a_confidence_outside_the_four_is_refused_by_the_parse() {
    // Arrange
    let off_vocabulary = format!("{PROVENANCE_BASE}
[[provenance]]
id = \"p\"
source = \"s\"
confidence = \"Probably\"
");
    // Act
    let refused = atlas_etl::sources::parse_sources(&off_vocabulary).expect_err("a confidence no member answers to must not even parse");
    // Assert
    assert_eq!(format!("{refused:#}"), CONFIDENCE_REFUSAL);
}

const ONE_INGESTED_SOURCE: &str = "## Per-source table\n\n| Source | License | Use |\n|---|---|---|\n| k, an ingested source | Public domain | Redistributed |\n";

const ONE_TYPEFACE: &str = "| Overpass typeface | SIL Open Font License 1.1 | Bundled, unmodified |\n";

#[test]
fn the_law_s_domain_is_the_ingested_sources_of_the_per_source_table_so_a_bundled_presentation_asset_is_outside_it() {
    // Arrange
    let doc = atlas_etl::sources::parse_sources(PROVENANCE_BASE).unwrap();
    let licenses_md = format!("{ONE_INGESTED_SOURCE}\n## Bundled presentation assets\n\n| Asset | License | Use |\n|---|---|---|\n{ONE_TYPEFACE}");
    // Act
    let reconciled = atlas_etl::sources::validate_against_licenses(&doc, &licenses_md);
    // Assert
    assert!(reconciled.is_ok(), "{reconciled:?}");
}

#[test]
fn a_presentation_asset_filed_among_the_ingested_sources_is_refused_and_pointed_at_its_own_table() {
    // Arrange
    let doc = atlas_etl::sources::parse_sources(PROVENANCE_BASE).unwrap();
    let licenses_md = format!("{ONE_INGESTED_SOURCE}{ONE_TYPEFACE}");
    // Act
    let refused = atlas_etl::sources::validate_against_licenses(&doc, &licenses_md).unwrap_err().to_string();
    // Assert
    assert!(refused.contains("Overpass typeface"), "{refused}");
    assert!(refused.contains("'## Bundled presentation assets'"), "{refused}");
}

#[test]
fn the_real_licenses_file_keeps_its_bundled_presentation_assets_out_of_the_per_source_table() {
    // Arrange
    let licenses_md = repo_root_file("LICENSES.md");
    // Act
    let per_source = licenses_md.find("## Per-source table").unwrap();
    let presentation = licenses_md.find("## Bundled presentation assets").expect("LICENSES.md carries its presentation assets in their own table");
    // Assert
    assert!(per_source < presentation);
    assert!(licenses_md[presentation..].contains("typeface"));
}
