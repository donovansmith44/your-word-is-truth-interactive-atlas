use std::path::{Path, PathBuf};

use atlas_graph::sqlite::extras::Extras;
use atlas_graph::sqlite::manifest::{
    raw_manifest_path, raw_provenance, read_manifest, write_manifest, Manifest, ManifestSection, RawProvenance,
    MANIFEST_SCHEMA,
};
use atlas_graph::sqlite::source::SectionLayout;
use atlas_graph::sqlite::writer::write_sections;
use atlas_graph::sqlite::SCHEMA_VERSION;
use atlas_graph_types::graph::Graph;
use atlas_graph_types::raw_manifest::RawHash;

const COMPILER: &str = "test";
const BUILT: &str = "2026-09-29T00:00:00Z";
const EARLIER_BUILT: &str = "2026-09-17T00:00:00Z";
const RAW_ROOT: RawHash = RawHash([0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef]);
const OTHER_RAW_ROOT: RawHash = RawHash([0xfe; 16]);
const RAW_ROOT_HEX: &str = "0123456789abcdef0123456789abcdef";
const OTHER_RAW_ROOT_HEX: &str = "fefefefefefefefefefefefefefefefe";
const NOT_A_HASH: &str = "zz";
const SECTION_LOGICAL: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const SECTION_BLOB: &str = "0000000000000000000000000000000000000000000000000000000000000000";
const SECTION_BYTES: u64 = 1;

#[test]
fn a_manifest_with_a_raw_root_is_spelled_with_one_raw_root_line_and_reads_back_whole() {
    // Arrange
    let manifest = one_section_manifest(Some(RAW_ROOT));
    let path = fresh_dir("with").join("manifest.toml");
    let expected_text = format!(
        "schema = {MANIFEST_SCHEMA}\ncompiler = \"{COMPILER}\"\nbuilt = \"{BUILT}\"\nroot = \"{}\"\nraw_root = \"{RAW_ROOT_HEX}\"\n\n{}",
        manifest.root,
        section_table()
    );

    // Act
    write_manifest(&manifest, &path).unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    let read = read_manifest(&path).unwrap();

    // Assert
    assert_eq!((text, read), (expected_text, manifest));
}

#[test]
fn a_manifest_without_a_raw_root_reads_back_without_one() {
    // Arrange
    let expected = one_section_manifest(None);
    let path = fresh_dir("without").join("manifest.toml");
    std::fs::write(
        &path,
        format!("schema = {MANIFEST_SCHEMA}\ncompiler = \"{COMPILER}\"\nbuilt = \"{BUILT}\"\nroot = \"{}\"\n\n{}", expected.root, section_table()),
    )
    .unwrap();

    // Act
    let read = read_manifest(&path).unwrap();

    // Assert
    assert_eq!(read, expected);
}

#[test]
fn a_raw_root_that_is_not_a_hash_is_refused_with_its_manifest() {
    // Arrange
    let root = one_section_manifest(None).root;
    let path = fresh_dir("bad-hex").join("manifest.toml");
    std::fs::write(
        &path,
        format!(
            "schema = {MANIFEST_SCHEMA}\ncompiler = \"{COMPILER}\"\nbuilt = \"{BUILT}\"\nroot = \"{root}\"\nraw_root = \"{NOT_A_HASH}\"\n\n{}",
            section_table()
        ),
    )
    .unwrap();
    let expected = format!(
        "manifest parse {}: TOML parse error at line 5, column 12\n  |\n5 | raw_root = \"{NOT_A_HASH}\"\n  |            ^^^^\nLength {{ expected: 32, found: 2 }}\n",
        path.display()
    );

    // Act
    let refused = read_manifest(&path).unwrap_err();

    // Assert
    assert_eq!(refused.to_string(), expected);
}

#[test]
fn the_raw_root_joins_the_manifest_without_moving_its_root_or_its_built() {
    // Arrange
    let graph = empty_graph();
    let layout = layout_in(&fresh_dir("joins"));
    let (without, _) = write_sections(&graph, &Extras::default(), COMPILER, &layout).unwrap();
    record_raw_root(&layout, RAW_ROOT_HEX);

    // Act
    let (with, _) = write_sections(&graph, &Extras::default(), COMPILER, &layout).unwrap();

    // Assert
    assert_eq!((without.raw_root, with), (None, Manifest { raw_root: Some(RAW_ROOT), ..without.clone() }));
}

#[test]
fn a_rewrite_with_the_raw_root_present_keeps_the_previous_built() {
    // Arrange
    let graph = empty_graph();
    let layout = layout_in(&fresh_dir("rewrite"));
    record_raw_root(&layout, RAW_ROOT_HEX);
    let (first, _) = write_sections(&graph, &Extras::default(), COMPILER, &layout).unwrap();
    let earlier = Manifest { built: EARLIER_BUILT.into(), ..first.clone() };
    write_manifest(&earlier, &layout.manifest_path()).unwrap();

    // Act
    let (rewritten, _) = write_sections(&graph, &Extras::default(), COMPILER, &layout).unwrap();

    // Assert
    assert_eq!(rewritten, earlier);
}

#[test]
fn a_raw_manifest_whose_root_is_not_a_hash_stops_the_writer() {
    // Arrange
    let layout = layout_in(&fresh_dir("writer-bad-hex"));
    record_raw_root(&layout, NOT_A_HASH);
    let expected = format!("raw manifest {}: root \"{NOT_A_HASH}\" is not a hash: Length {{ expected: 32, found: 2 }}", raw_manifest_path(&layout).display());

    // Act
    let stopped = write_sections(&empty_graph(), &Extras::default(), COMPILER, &layout).unwrap_err();

    // Assert
    assert_eq!(stopped.to_string(), expected);
}

#[test]
fn a_raw_manifest_that_is_not_toml_stops_the_writer() {
    // Arrange
    let layout = layout_in(&fresh_dir("writer-not-toml"));
    let path = raw_manifest_path(&layout);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, "root = \n").unwrap();
    let expected = format!("raw manifest parse {}: TOML parse error at line 1, column 8\n  |\n1 | root = \n  |        ^\ninvalid string\nexpected `\"`, `'`\n", path.display());

    // Act
    let stopped = write_sections(&empty_graph(), &Extras::default(), COMPILER, &layout).unwrap_err();

    // Assert
    assert_eq!(stopped.to_string(), expected);
}

#[test]
fn provenance_says_the_inputs_did_not_move_when_both_sides_record_the_same_root() {
    // Arrange
    let expected = format!(
        "the raw inputs did not move: data/raw/MANIFEST.toml records {RAW_ROOT_HEX}, the root the compiled artifact was built from, so the difference is in the code (or in a file 'bibex verify' would name)"
    );

    // Act
    let provenance = raw_provenance(Some(RAW_ROOT), Some(RAW_ROOT));

    // Assert
    assert_eq!((provenance.to_string(), provenance), (expected, RawProvenance::Unchanged(RAW_ROOT)));
}

#[test]
fn provenance_says_the_inputs_also_moved_when_the_recorded_root_differs_from_the_compiled_one() {
    // Arrange
    let expected = format!(
        "the raw inputs ALSO moved: the compiled artifact was built from raw {RAW_ROOT_HEX} but data/raw/MANIFEST.toml records {OTHER_RAW_ROOT_HEX} -- a refetch or 'bibex raw bless' changed the inputs since the artifact was compiled"
    );

    // Act
    let provenance = raw_provenance(Some(RAW_ROOT), Some(OTHER_RAW_ROOT));

    // Assert
    assert_eq!(
        (provenance.to_string(), provenance),
        (expected, RawProvenance::Changed { compiled_from: RAW_ROOT, recorded: OTHER_RAW_ROOT })
    );
}

#[test]
fn provenance_is_unrecorded_when_either_side_is_silent_and_says_which() {
    // Arrange
    let expected = format!("raw provenance unrecorded: the compiled artifact names no raw root, data/raw/MANIFEST.toml records {OTHER_RAW_ROOT_HEX}");

    // Act
    let provenance = raw_provenance(None, Some(OTHER_RAW_ROOT));

    // Assert
    assert_eq!(
        (provenance.to_string(), provenance),
        (expected, RawProvenance::Unrecorded { compiled_from: None, recorded: Some(OTHER_RAW_ROOT) })
    );
}

#[test]
fn provenance_is_unrecorded_when_the_tree_was_never_blessed_and_says_what_the_artifact_names() {
    // Arrange
    let expected = format!("raw provenance unrecorded: the compiled artifact was built from raw {RAW_ROOT_HEX}, data/raw/MANIFEST.toml records none");

    // Act
    let provenance = raw_provenance(Some(RAW_ROOT), None);

    // Assert
    assert_eq!(
        (provenance.to_string(), provenance),
        (expected, RawProvenance::Unrecorded { compiled_from: Some(RAW_ROOT), recorded: None })
    );
}

fn one_section_manifest(raw_root: Option<RawHash>) -> Manifest {
    let sections = vec![ManifestSection {
        name: "core".into(),
        required: true,
        logical: SECTION_LOGICAL.into(),
        blob: SECTION_BLOB.into(),
        bytes: SECTION_BYTES,
        schema_version: SCHEMA_VERSION,
    }];
    Manifest {
        schema: MANIFEST_SCHEMA,
        compiler: COMPILER.into(),
        built: BUILT.into(),
        root: atlas_graph::sqlite::manifest::root_of(&sections),
        raw_root,
        sections,
    }
}

fn section_table() -> String {
    format!(
        "[[section]]\nname = \"core\"\nrequired = true\nlogical = \"{SECTION_LOGICAL}\"\nblob = \"{SECTION_BLOB}\"\nbytes = {SECTION_BYTES}\nschema_version = {SCHEMA_VERSION}\n"
    )
}

fn empty_graph() -> Graph {
    let mut graph = Graph::default();
    graph.build_indexes();
    graph
}

fn fresh_dir(test: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("raw-provenance-{}-{test}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn layout_in(dir: &Path) -> SectionLayout {
    SectionLayout::under(&dir.join("data").join("compiled"))
}

fn record_raw_root(layout: &SectionLayout, root: &str) {
    let path = raw_manifest_path(layout);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, format!("schema = 1\nroot = \"{root}\"\n")).unwrap();
}
