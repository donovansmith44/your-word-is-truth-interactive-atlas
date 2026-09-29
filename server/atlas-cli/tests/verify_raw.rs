use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};

use atlas_cli::raw::{walk, write_manifest, MANIFEST_FILE};
use atlas_graph::sqlite::manifest::{root_of, write_manifest as write_compiled_manifest, Manifest, ManifestSection, MANIFEST_SCHEMA};
use atlas_graph_types::raw_manifest::Sha256;
use atlas_graph_types::sha256::sha256;
use serde_json::{json, Value};

const ANCIENT: &[u8] = b"ancient places\n";
const MODERN: &[u8] = b"modern places\n";
const VOLUME_ONE: &[u8] = b"kretzmann volume one\n";
const ANCIENT_FLIPPED: &[u8] = b"Ancient places\n";
const ANCIENT_TRUNCATED: &[u8] = b"ancient p";
const ANCIENT_GROWN: &[u8] = b"ancient places and more\n";
const ADDED: &[u8] = b"added\n";
const FIXTURE_ROOT: &str = "eb1eaed77201691558ffd5df9731229b";
const FIXTURE_FILES: usize = 3;
const ANCIENT_SHA256: &str = "fca704c93e62cc48ceea501492fe072215bcf717ffccc7a69d37226f9cbee568";

const OPTIONAL_SECTION: &str = "concord";
const REQUIRED_SECTION: &str = "core";
const ZERO_LOGICAL: &str = "00000000000000000000000000000000";
const ZERO_BLOB: &str = "0000000000000000000000000000000000000000000000000000000000000000";
const SECTION_SCHEMA: u32 = 14;
const OPTIONAL_SECTION_LINE: &str = "concord    logical 00000000000000000000000000000000  skipped  transport absent (optional) 0 bytes\n";

const EXIT_OK: i32 = 0;
const EXIT_INTEGRITY_FAILED: i32 = 6;

const DO_COMPILED: &str = "recompile (cargo run -p atlas-graph --bin atlas-graph-compile, from server/) or restore data/compiled from git; a tampered or truncated section must never be served";
const DO_RAW: &str = "restore data/raw from the archive under Documents/bible-atlas-backups or refetch it with data/fetch-raw.ps1; if the change was deliberate, record it with 'bibex raw bless' and commit data/raw/MANIFEST.toml";

struct Fixture {
    dir: PathBuf,
}

impl Fixture {
    fn named(test: &str) -> Fixture {
        let dir = std::env::temp_dir().join(format!("verify-raw-{}-{test}", std::process::id())).join("data");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("compiled")).expect("the fixture directory must be creatable");
        let fixture = Fixture { dir };
        fixture.record_compiled(OPTIONAL_SECTION, false);
        fixture
    }

    fn compiled(&self) -> PathBuf {
        self.dir.join("compiled")
    }

    fn raw(&self) -> PathBuf {
        self.dir.join("raw")
    }

    fn manifest_path(&self) -> PathBuf {
        self.raw().join(MANIFEST_FILE)
    }

    fn compiled_root(&self) -> String {
        root_of(&[section(OPTIONAL_SECTION, false)])
    }

    fn record_compiled(&self, name: &str, required: bool) {
        let sections = vec![section(name, required)];
        let manifest = Manifest {
            schema: MANIFEST_SCHEMA,
            compiler: "verify-raw fixture".into(),
            built: "2026-09-29T00:00:00Z".into(),
            root: root_of(&sections),
            sections,
        };
        write_compiled_manifest(&manifest, &self.compiled().join("manifest.toml")).expect("the compiled manifest must be writable");
    }

    fn write(&self, relative: &str, bytes: &[u8]) {
        let path = self.raw().join(relative);
        fs::create_dir_all(path.parent().expect("every fixture file sits in a directory")).expect("fixture directories must be creatable");
        fs::write(path, bytes).expect("fixture files must be writable");
    }

    fn make_dir(&self, relative: &str) {
        fs::create_dir_all(self.raw().join(relative)).expect("fixture directories must be creatable");
    }

    fn remove(&self, relative: &str) {
        fs::remove_file(self.raw().join(relative)).expect("fixture files must be removable");
    }

    fn remove_dir(&self, relative: &str) {
        fs::remove_dir_all(self.raw().join(relative)).expect("fixture directories must be removable");
    }

    fn record(&self) {
        let manifest = walk(&self.raw()).expect("the fixture tree must walk");
        write_manifest(&self.manifest_path(), &manifest).expect("the raw manifest must be writable");
    }

    fn manifest_text(&self) -> String {
        fs::read_to_string(self.manifest_path()).expect("the raw manifest must be readable")
    }

    fn root_line(&self) -> String {
        format!("root {} OK (recomputed from 1 section lines)\n", self.compiled_root())
    }

    fn compiled_json(&self) -> Value {
        json!({
            "root": { "manifest": self.compiled_root(), "recomputed": self.compiled_root(), "ok": true },
            "sections": [{
                "name": OPTIONAL_SECTION,
                "required": false,
                "logical": ZERO_LOGICAL,
                "blob": ZERO_BLOB,
                "bytes": 0,
                "transport": "absent",
                "logical_check": "skipped",
                "schema_version": SECTION_SCHEMA,
                "uncompressed_bytes": null,
            }],
        })
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(self.dir.parent().expect("the fixture data directory has a parent"));
    }
}

fn section(name: &str, required: bool) -> ManifestSection {
    ManifestSection { name: name.into(), required, logical: ZERO_LOGICAL.into(), blob: ZERO_BLOB.into(), bytes: 0, schema_version: SECTION_SCHEMA }
}

fn recorded_fixture(test: &str) -> Fixture {
    let fixture = Fixture::named(test);
    fixture.write("kretzmann/volume-1.txt", VOLUME_ONE);
    fixture.write("geo/modern.jsonl", MODERN);
    fixture.write("geo/ancient.jsonl", ANCIENT);
    fixture.make_dir("empty");
    fixture.record();
    fixture
}

fn bibex(fixture: &Fixture, args: &[&str]) -> (Option<i32>, String, String) {
    let compiled = fixture.compiled();
    let mut full = vec!["--data-dir", compiled.to_str().expect("the data dir path must be valid UTF-8")];
    full.extend_from_slice(args);
    let output: Output = Command::new(env!("CARGO_BIN_EXE_bibex")).args(&full).output().expect("bibex must run");
    (
        output.status.code(),
        String::from_utf8(output.stdout).expect("stdout must be valid UTF-8"),
        String::from_utf8(output.stderr).expect("stderr must be valid UTF-8"),
    )
}

fn bibex_json(fixture: &Fixture, args: &[&str]) -> (Option<i32>, Value, String) {
    let mut full = vec!["--json"];
    full.extend_from_slice(args);
    let (code, stdout, stderr) = bibex(fixture, &full);
    (code, serde_json::from_str(&stdout).expect("--json stdout must be one JSON value"), stderr)
}

fn hex_of(bytes: &[u8]) -> String {
    Sha256(sha256(bytes)).hex()
}

fn integrity_failed(failed: usize, checks: usize, failures: &str, do_: &str) -> String {
    format!("atlas: error (integrity_failed): {failed} of {checks} checks failed -- {failures} -- {do_}\n")
}

#[test]
fn verify_prints_the_raw_root_when_the_tree_matches_its_manifest() {
    // Arrange
    let fixture = recorded_fixture("matching");
    let expected = format!("{OPTIONAL_SECTION_LINE}{}raw {FIXTURE_ROOT} OK ({FIXTURE_FILES} files)\n", fixture.root_line());

    // Act
    let result = bibex(&fixture, &["verify"]);

    // Assert
    assert_eq!(result, (Some(EXIT_OK), expected, String::new()));
}

#[test]
fn verify_json_carries_the_raw_root() {
    // Arrange
    let fixture = recorded_fixture("matching-json");
    let mut expected = fixture.compiled_json();
    expected["raw"] = json!({ "status": "ok", "root": FIXTURE_ROOT, "files": FIXTURE_FILES });

    // Act
    let result = bibex_json(&fixture, &["verify"]);

    // Assert
    assert_eq!(result, (Some(EXIT_OK), expected, String::new()));
}

#[test]
fn a_flipped_byte_exits_6_and_names_the_file_with_both_hashes() {
    // Arrange
    let fixture = recorded_fixture("flipped");
    fixture.write("geo/ancient.jsonl", ANCIENT_FLIPPED);
    let failure = format!("raw geo/ancient.jsonl: MISMATCH manifest {ANCIENT_SHA256} file {}", hex_of(ANCIENT_FLIPPED));

    // Act
    let result = bibex(&fixture, &["verify"]);

    // Assert
    assert_eq!(result, (Some(EXIT_INTEGRITY_FAILED), String::new(), integrity_failed(1, 4, &failure, DO_RAW)));
}

#[test]
fn a_missing_file_is_named_as_missing() {
    // Arrange
    let fixture = recorded_fixture("missing");
    fixture.remove("kretzmann/volume-1.txt");

    // Act
    let result = bibex(&fixture, &["verify"]);

    // Assert
    assert_eq!(result, (Some(EXIT_INTEGRITY_FAILED), String::new(), integrity_failed(1, 4, "raw kretzmann/volume-1.txt: MISSING", DO_RAW)));
}

#[test]
fn an_extra_file_is_named_as_extra() {
    // Arrange
    let fixture = recorded_fixture("extra");
    fixture.write("geo/added.jsonl", ADDED);

    // Act
    let result = bibex(&fixture, &["verify"]);

    // Assert
    assert_eq!(result, (Some(EXIT_INTEGRITY_FAILED), String::new(), integrity_failed(1, 4, "raw geo/added.jsonl: EXTRA (not in MANIFEST.toml)", DO_RAW)));
}

#[test]
fn a_truncated_file_is_named_with_both_sizes() {
    // Arrange
    let fixture = recorded_fixture("truncated");
    fixture.write("geo/ancient.jsonl", ANCIENT_TRUNCATED);
    let failure = format!("raw geo/ancient.jsonl: TRUNCATED manifest {} bytes file {}", ANCIENT.len(), ANCIENT_TRUNCATED.len());

    // Act
    let result = bibex(&fixture, &["verify"]);

    // Assert
    assert_eq!(result, (Some(EXIT_INTEGRITY_FAILED), String::new(), integrity_failed(1, 4, &failure, DO_RAW)));
}

#[test]
fn a_file_that_grew_is_a_mismatch_not_a_truncation() {
    // Arrange
    let fixture = recorded_fixture("grown");
    fixture.write("geo/ancient.jsonl", ANCIENT_GROWN);
    let failure = format!("raw geo/ancient.jsonl: MISMATCH manifest {ANCIENT_SHA256} file {}", hex_of(ANCIENT_GROWN));

    // Act
    let result = bibex(&fixture, &["verify"]);

    // Assert
    assert_eq!(result, (Some(EXIT_INTEGRITY_FAILED), String::new(), integrity_failed(1, 4, &failure, DO_RAW)));
}

#[test]
fn a_file_where_a_directory_was_recorded_is_missing_and_extra_at_once() {
    // Arrange
    let fixture = recorded_fixture("kind-swap");
    fixture.remove_dir("empty");
    fixture.write("empty", ADDED);

    // Act
    let result = bibex(&fixture, &["verify"]);

    // Assert
    assert_eq!(result, (Some(EXIT_INTEGRITY_FAILED), String::new(), integrity_failed(1, 4, "raw empty: MISSING; raw empty: EXTRA (not in MANIFEST.toml)", DO_RAW)));
}

#[test]
fn no_manifest_is_unrecorded_and_the_exit_code_is_untouched() {
    // Arrange
    let fixture = Fixture::named("no-manifest");
    fixture.write("geo/ancient.jsonl", ANCIENT);
    let expected = format!("{OPTIONAL_SECTION_LINE}{}raw: unrecorded (no MANIFEST.toml at {})\n", fixture.root_line(), fixture.manifest_path().display());

    // Act
    let result = bibex(&fixture, &["verify"]);

    // Assert
    assert_eq!(result, (Some(EXIT_OK), expected, String::new()));
}

#[test]
fn no_manifest_is_unrecorded_in_json_too() {
    // Arrange
    let fixture = Fixture::named("no-manifest-json");
    fixture.write("geo/ancient.jsonl", ANCIENT);
    let mut expected = fixture.compiled_json();
    expected["raw"] = json!({ "status": "unrecorded", "why": format!("no MANIFEST.toml at {}", fixture.manifest_path().display()) });

    // Act
    let result = bibex_json(&fixture, &["verify"]);

    // Assert
    assert_eq!(result, (Some(EXIT_OK), expected, String::new()));
}

#[test]
fn an_empty_tree_is_unrecorded_even_with_a_manifest_never_a_pass() {
    // Arrange
    let fixture = recorded_fixture("emptied");
    fixture.remove_dir("geo");
    fixture.remove_dir("kretzmann");
    let expected = format!("{OPTIONAL_SECTION_LINE}{}raw: unrecorded ({} has no files)\n", fixture.root_line(), fixture.raw().display());

    // Act
    let result = bibex(&fixture, &["verify"]);

    // Assert
    assert_eq!(result, (Some(EXIT_OK), expected, String::new()));
}

#[test]
fn a_manifest_that_does_not_recompute_is_an_integrity_failure() {
    // Arrange
    let fixture = recorded_fixture("bad-manifest");
    let forged = fixture.manifest_text().replace(FIXTURE_ROOT, ZERO_LOGICAL);
    fs::write(fixture.manifest_path(), forged).expect("the forged manifest must be writable");
    let expected = format!(
        "atlas: error (integrity_failed): {} does not verify -- manifest root {ZERO_LOGICAL} does not recompute from its datasets ({FIXTURE_ROOT}) -- {DO_RAW}\n",
        fixture.manifest_path().display()
    );

    // Act
    let result = bibex(&fixture, &["verify"]);

    // Assert
    assert_eq!(result, (Some(EXIT_INTEGRITY_FAILED), String::new(), expected));
}

#[test]
fn a_compiled_failure_and_a_raw_failure_carry_both_remedies() {
    // Arrange
    let fixture = recorded_fixture("both");
    fixture.record_compiled(REQUIRED_SECTION, true);
    fixture.remove("geo/ancient.jsonl");
    let blob = fixture.compiled().join("sections").join(format!("{REQUIRED_SECTION}.{ZERO_LOGICAL}.sqlite.zst"));
    let failures = format!("{REQUIRED_SECTION}: required blob MISSING at {}; raw geo/ancient.jsonl: MISSING", blob.display());
    let do_ = format!("{DO_COMPILED}; {DO_RAW}");

    // Act
    let result = bibex(&fixture, &["verify"]);

    // Assert
    assert_eq!(result, (Some(EXIT_INTEGRITY_FAILED), String::new(), integrity_failed(2, 4, &failures, &do_)));
}

#[test]
fn a_compiled_failure_alone_keeps_the_compiled_remedy() {
    // Arrange
    let fixture = recorded_fixture("compiled-only");
    fixture.record_compiled(REQUIRED_SECTION, true);
    let blob = fixture.compiled().join("sections").join(format!("{REQUIRED_SECTION}.{ZERO_LOGICAL}.sqlite.zst"));
    let failure = format!("{REQUIRED_SECTION}: required blob MISSING at {}", blob.display());

    // Act
    let result = bibex(&fixture, &["verify"]);

    // Assert
    assert_eq!(result, (Some(EXIT_INTEGRITY_FAILED), String::new(), integrity_failed(1, 4, &failure, DO_COMPILED)));
}

#[test]
fn section_verifies_only_that_compiled_section_and_leaves_raw_unasked() {
    // Arrange
    let fixture = recorded_fixture("section");
    fixture.write("geo/ancient.jsonl", ANCIENT_FLIPPED);
    let expected = format!("{OPTIONAL_SECTION_LINE}{}", fixture.root_line());

    // Act
    let result = bibex(&fixture, &["verify", "--section", OPTIONAL_SECTION]);

    // Assert
    assert_eq!(result, (Some(EXIT_OK), expected, String::new()));
}

#[test]
fn section_json_carries_no_raw_answer() {
    // Arrange
    let fixture = recorded_fixture("section-json");
    let mut expected = fixture.compiled_json();
    expected["raw"] = Value::Null;

    // Act
    let result = bibex_json(&fixture, &["verify", "--section", OPTIONAL_SECTION]);

    // Assert
    assert_eq!(result, (Some(EXIT_OK), expected, String::new()));
}

/// Windows only: an exclusive open (share mode 0) is the one portable way this test can make a
/// file unreadable without elevation; other hosts have no share modes.
#[cfg(windows)]
#[test]
fn a_tree_the_walk_cannot_read_is_an_integrity_failure() {
    // Arrange
    use std::os::windows::fs::OpenOptionsExt;
    let fixture = recorded_fixture("unreadable");
    let locked = fixture.raw().join("geo").join("ancient.jsonl");
    let _hold = fs::OpenOptions::new().read(true).share_mode(0).open(&locked).expect("the fixture file must open exclusively");
    let why = fs::read(&locked).expect_err("an exclusively held file must not read").to_string();
    let expected = format!("atlas: error (integrity_failed): {} could not be walked -- {why} -- {DO_RAW}\n", fixture.raw().display());

    // Act
    let result = bibex(&fixture, &["verify"]);

    // Assert
    assert_eq!(result, (Some(EXIT_INTEGRITY_FAILED), String::new(), expected));
}
