use std::path::{Path, PathBuf};
use std::process::Command;

const COMPILER: &str = env!("CARGO_BIN_EXE_atlas-etl");
const SOURCES_GENERATOR: &str = env!("CARGO_BIN_EXE_gen_sources");
const CLEAN_EXIT: Option<i32> = Some(0);

mod common;

use common::{curated_dir, raw_dir};

/// Both binaries resolve their inputs relative to the working directory, which is
/// `server/` for every documented invocation.
fn server_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}


#[test]
fn the_compiler_prints_exactly_the_report_the_library_derives_from_the_same_trees() {
    // Arrange
    let expected = atlas_etl::report::write(
        &atlas_etl::compile::compile(&raw_dir(), &curated_dir()).expect("the committed trees must compile").report,
    );

    // Act
    let run = Command::new(COMPILER).current_dir(server_dir()).output().expect("the atlas-etl binary must run");

    // Assert
    assert_eq!(
        (run.status.code(), String::from_utf8_lossy(&run.stdout).replace("\r\n", "\n")),
        (CLEAN_EXIT, expected)
    );
}

#[test]
fn the_sources_generator_rewrites_the_compiled_registry_byte_for_byte_and_says_what_it_wrote() {
    // Arrange
    let compiled_path = Path::new("..").join("data").join("compiled").join("sources.json");
    let committed = std::fs::read(server_dir().join(&compiled_path)).expect("data/compiled/sources.json must exist");
    let doc = atlas_etl::sources::parse_sources(
        &std::fs::read_to_string(curated_dir().join("sources.toml")).expect("data/curated/sources.toml must exist"),
    )
    .expect("data/curated/sources.toml must parse");
    let expected_report = format!(
        "gen_sources: wrote {} categories, {} sources, {} provenance rows to {} (validated 1:1 against LICENSES.md's per-source table)\n",
        doc.categories.len(),
        doc.sources.len(),
        doc.provenances.len(),
        compiled_path.display()
    );

    // Act
    let run = Command::new(SOURCES_GENERATOR).current_dir(server_dir()).output().expect("the gen_sources binary must run");

    // Assert
    assert_eq!(
        (
            run.status.code(),
            String::from_utf8_lossy(&run.stdout).replace("\r\n", "\n"),
            std::fs::read(server_dir().join(&compiled_path)).expect("data/compiled/sources.json must still exist") == committed
        ),
        (CLEAN_EXIT, expected_report, true)
    );
}
