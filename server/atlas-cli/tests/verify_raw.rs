use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};

use atlas_cli::raw::{walk, write_manifest, MANIFEST_FILE};
use atlas_graph::sqlite::manifest::{root_of, write_manifest as write_compiled_manifest, Manifest, ManifestSection, MANIFEST_SCHEMA};
use atlas_graph_types::raw_manifest::{RawEntry, RawHash, Sha256};
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
const OTHER_RAW_ROOT: &str = "ffffffffffffffffffffffffffffffff";
const ANCIENT_SHA256: &str = "fca704c93e62cc48ceea501492fe072215bcf717ffccc7a69d37226f9cbee568";

const OPTIONAL_SECTION: &str = "concord";
const REQUIRED_SECTION: &str = "core";
const ZERO_LOGICAL: &str = "00000000000000000000000000000000";
const ZERO_BLOB: &str = "0000000000000000000000000000000000000000000000000000000000000000";
const SECTION_SCHEMA: u32 = atlas_graph::sqlite::SCHEMA_VERSION;
const OPTIONAL_SECTION_LINE: &str = "concord    logical 00000000000000000000000000000000  skipped  transport absent (optional) 0 bytes\n";

const EXIT_OK: i32 = 0;
const EXIT_DATA_LOAD_FAILED: i32 = 5;
const EXIT_INTEGRITY_FAILED: i32 = 6;
const EXIT_BAD_USAGE: i32 = 4;
const EXIT_NOT_FOUND: i32 = 3;

const DO_COMPILED: &str = "recompile (cargo run -p atlas-graph --bin atlas-graph-compile, from server/) or restore data/compiled from git; a tampered or truncated section must never be served";
const DO_RAW: &str = "restore data/raw from the archive under Documents/bible-atlas-backups or refetch it with data/fetch-raw.ps1; if the change was deliberate, record it with 'bibex raw bless' and commit data/raw/MANIFEST.toml";
const DO_BLESS_EMPTY: &str = "run data/fetch-raw.ps1 (or restore data/raw from the archive), check it with 'bibex verify', then bless";
const DO_BLESS_UNREADABLE: &str = "restore data/raw/MANIFEST.toml from git, or delete it deliberately, then bless again";
const DO_BLESS_UNWALKABLE: &str = "pass --data-dir so that data/raw sits beside it, and fetch the raw tree first with data/fetch-raw.ps1";
const DO_BLESS_UNWRITABLE: &str = "check that data/raw is writable";
const DO_RAW_VERB: &str = "run 'bibex raw bless' to record data/raw in data/raw/MANIFEST.toml, or 'bibex raw check <path>' to ask whether one path is as recorded";
const RAW_USAGE: &str = "usage: bibex raw bless | bibex raw check <path>";
const WHY_UNRECORDED: &str = "MANIFEST.toml has no file or directory at that path";
const DO_UNRECORDED: &str = "fetch it, then record it with 'bibex raw bless' and commit data/raw/MANIFEST.toml";
const WHY_BAD_PATH: &str = "'raw check' takes one path relative to data/raw: plain names joined by a path separator, no '.', '..' or root";
const DO_BAD_PATH: &str = "name a file or directory as MANIFEST.toml spells it, e.g. kjv.json or brain-fuel-bible/lexicon";

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
        self.record_compiled_with(name, required, None);
    }

    fn record_compiled_from(&self, raw_root: &str) {
        self.record_compiled_with(OPTIONAL_SECTION, false, Some(RawHash::parse(raw_root).expect("a fixture raw root is hex")));
    }

    fn record_compiled_with(&self, name: &str, required: bool, raw_root: Option<RawHash>) {
        let sections = vec![section(name, required)];
        let manifest = Manifest {
            schema: MANIFEST_SCHEMA,
            compiler: "verify-raw fixture".into(),
            built: "2026-09-29T00:00:00Z".into(),
            root: root_of(&sections),
            raw_root,
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
            "root": { "manifest": self.compiled_root(), "recomputed": self.compiled_root(), "ok": true, "raw_root": null },
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

/// Windows only: `mklink /J` needs no elevation, so it runs on every Windows host; other hosts
/// have no junctions. The target lives inside the fixture, so a worst-case follow could only
/// reach the test's own files.
#[cfg(windows)]
impl Fixture {
    fn junction(&self, relative: &str) -> PathBuf {
        let elsewhere = self.dir.join("elsewhere");
        fs::create_dir_all(&elsewhere).expect("the junction target must be creatable");
        let link = relative.split('/').fold(self.raw(), |path, component| path.join(component));
        let made = Command::new("cmd")
            .args(["/C", "mklink", "/J", link.to_str().expect("utf-8 path"), elsewhere.to_str().expect("utf-8 path")])
            .output()
            .expect("cmd must run");
        assert!(made.status.success(), "mklink /J: {}", String::from_utf8_lossy(&made.stderr));
        link
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

fn data_load_failed(what: &str, why: &str, do_: &str) -> String {
    format!("atlas: error (data_load_failed): {what} -- {why} -- {do_}\n")
}

fn bad_usage(what: &str, why: &str, do_: &str) -> String {
    format!("atlas: error (bad_usage): {what} -- {why} -- {do_}\n")
}

fn walked_root(fixture: &Fixture) -> String {
    walk(&fixture.raw()).expect("the fixture tree must walk").root.hex()
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
fn verify_says_the_sections_were_compiled_from_the_tree_it_walked() {
    // Arrange
    let fixture = recorded_fixture("compiled-from-it");
    fixture.record_compiled_from(FIXTURE_ROOT);
    let expected = format!("{OPTIONAL_SECTION_LINE}{}raw {FIXTURE_ROOT} OK ({FIXTURE_FILES} files; the sections were compiled from it)
", fixture.root_line());

    // Act
    let result = bibex(&fixture, &["verify"]);

    // Assert
    assert_eq!(result, (Some(EXIT_OK), expected, String::new()));
}

#[test]
fn verify_names_the_other_tree_the_sections_were_compiled_from() {
    // Arrange
    let fixture = recorded_fixture("compiled-from-another");
    fixture.record_compiled_from(OTHER_RAW_ROOT);
    let expected = format!(
        "{OPTIONAL_SECTION_LINE}{}raw {FIXTURE_ROOT} OK ({FIXTURE_FILES} files; the sections were compiled from raw {OTHER_RAW_ROOT}, not from this tree)
",
        fixture.root_line()
    );

    // Act
    let result = bibex(&fixture, &["verify"]);

    // Assert
    assert_eq!(result, (Some(EXIT_OK), expected, String::new()));
}

#[test]
fn verify_json_carries_the_raw_root_the_sections_were_compiled_from() {
    // Arrange
    let fixture = recorded_fixture("compiled-from-json");
    fixture.record_compiled_from(FIXTURE_ROOT);
    let mut expected = fixture.compiled_json();
    expected["root"]["raw_root"] = json!(FIXTURE_ROOT);
    expected["raw"] = json!({ "status": "ok", "root": FIXTURE_ROOT, "files": FIXTURE_FILES });

    // Act
    let result = bibex_json(&fixture, &["verify"]);

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
fn a_file_the_walk_cannot_read_is_named_in_the_integrity_failure() {
    // Arrange
    use std::os::windows::fs::OpenOptionsExt;
    let fixture = recorded_fixture("unreadable");
    let locked = fixture.raw().join("geo").join("ancient.jsonl");
    let _hold = fs::OpenOptions::new().read(true).share_mode(0).open(&locked).expect("the fixture file must open exclusively");
    let why = fs::read(&locked).expect_err("an exclusively held file must not read").to_string();
    let expected = format!("atlas: error (integrity_failed): {} could not be walked -- {}: {why} -- {DO_RAW}\n", fixture.raw().display(), locked.display());

    // Act
    let result = bibex(&fixture, &["verify"]);

    // Assert
    assert_eq!(result, (Some(EXIT_INTEGRITY_FAILED), String::new(), expected));
}

#[cfg(windows)]
#[test]
fn a_junction_the_manifest_does_not_list_is_named_as_a_link() {
    // Arrange
    let fixture = recorded_fixture("link-added");
    let link = fixture.junction("geo/linked");

    // Act
    let result = bibex(&fixture, &["verify"]);
    fs::remove_dir(&link).expect("the junction must be removable on its own");

    // Assert
    assert_eq!(result, (Some(EXIT_INTEGRITY_FAILED), String::new(), integrity_failed(1, 4, "raw geo/linked: LINK (a junction or symlink MANIFEST.toml does not list)", DO_RAW)));
}

#[cfg(windows)]
#[test]
fn a_junction_the_manifest_lists_but_the_tree_has_lost_is_named_as_a_missing_link() {
    // Arrange
    let fixture = recorded_fixture("link-removed");
    let link = fixture.junction("geo/linked");
    fixture.record();
    fs::remove_dir(&link).expect("the junction must be removable on its own");

    // Act
    let result = bibex(&fixture, &["verify"]);

    // Assert
    assert_eq!(result, (Some(EXIT_INTEGRITY_FAILED), String::new(), integrity_failed(1, 4, "raw geo/linked: LINK MISSING (MANIFEST.toml lists a junction or symlink there)", DO_RAW)));
}

#[test]
fn raw_bless_records_the_tree_and_says_there_was_no_previous_manifest() {
    // Arrange
    let fixture = Fixture::named("bless-first");
    fixture.write("kretzmann/volume-1.txt", VOLUME_ONE);
    fixture.write("geo/modern.jsonl", MODERN);
    fixture.write("geo/ancient.jsonl", ANCIENT);
    fixture.make_dir("empty");
    let expected = format!("raw {FIXTURE_ROOT} recorded ({FIXTURE_FILES} files) at {}\nprevious none\n", fixture.manifest_path().display());

    // Act
    let result = bibex(&fixture, &["raw", "bless"]);

    // Assert
    assert_eq!(result, (Some(EXIT_OK), expected, String::new()));
    assert_eq!(bibex(&fixture, &["verify"]), (Some(EXIT_OK), format!("{OPTIONAL_SECTION_LINE}{}raw {FIXTURE_ROOT} OK ({FIXTURE_FILES} files)\n", fixture.root_line()), String::new()));
}

#[test]
fn raw_bless_prints_added_removed_and_changed_against_the_previous_manifest() {
    // Arrange
    let fixture = recorded_fixture("bless-diff");
    fixture.write("geo/added.jsonl", ADDED);
    fixture.write("geo/ancient.jsonl", ANCIENT_TRUNCATED);
    fixture.remove("kretzmann/volume-1.txt");
    let expected = format!(
        "raw {} recorded ({FIXTURE_FILES} files) at {}\nprevious {FIXTURE_ROOT}\nadded geo/added.jsonl\nremoved kretzmann/volume-1.txt\nchanged geo/ancient.jsonl\n",
        walked_root(&fixture),
        fixture.manifest_path().display()
    );

    // Act
    let result = bibex(&fixture, &["raw", "bless"]);

    // Assert
    assert_eq!(result, (Some(EXIT_OK), expected, String::new()));
    assert_eq!(bibex(&fixture, &["verify"]), (Some(EXIT_OK), format!("{OPTIONAL_SECTION_LINE}{}raw {} OK ({FIXTURE_FILES} files)\n", fixture.root_line(), walked_root(&fixture)), String::new()));
}

#[test]
fn raw_bless_says_unchanged_when_nothing_moved() {
    // Arrange
    let fixture = recorded_fixture("bless-unchanged");
    let expected = format!("raw {FIXTURE_ROOT} recorded ({FIXTURE_FILES} files) at {}\nprevious {FIXTURE_ROOT}\nunchanged\n", fixture.manifest_path().display());

    // Act
    let result = bibex(&fixture, &["raw", "bless"]);

    // Assert
    assert_eq!(result, (Some(EXIT_OK), expected, String::new()));
}

#[test]
fn raw_bless_json_carries_the_root_and_the_diff() {
    // Arrange
    let fixture = recorded_fixture("bless-json");
    fixture.write("geo/added.jsonl", ADDED);
    fixture.write("geo/ancient.jsonl", ANCIENT_FLIPPED);
    fixture.remove("kretzmann/volume-1.txt");
    let expected = json!({
        "root": walked_root(&fixture),
        "files": FIXTURE_FILES,
        "manifest": fixture.manifest_path().display().to_string(),
        "previous": FIXTURE_ROOT,
        "added": ["geo/added.jsonl"],
        "removed": ["kretzmann/volume-1.txt"],
        "changed": ["geo/ancient.jsonl"],
    });

    // Act
    let result = bibex_json(&fixture, &["raw", "bless"]);

    // Assert
    assert_eq!(result, (Some(EXIT_OK), expected, String::new()));
}

#[test]
fn raw_bless_json_says_previous_null_on_a_first_blessing() {
    // Arrange
    let fixture = Fixture::named("bless-json-first");
    fixture.write("geo/ancient.jsonl", ANCIENT);
    let expected = json!({
        "root": walked_root(&fixture),
        "files": 1,
        "manifest": fixture.manifest_path().display().to_string(),
        "previous": null,
        "added": [],
        "removed": [],
        "changed": [],
    });

    // Act
    let result = bibex_json(&fixture, &["raw", "bless"]);

    // Assert
    assert_eq!(result, (Some(EXIT_OK), expected, String::new()));
}

#[test]
fn raw_bless_refuses_an_empty_tree_and_leaves_the_previous_manifest_untouched() {
    // Arrange
    let fixture = recorded_fixture("bless-empty");
    let previous = fixture.manifest_text();
    fixture.remove_dir("geo");
    fixture.remove_dir("kretzmann");
    let expected = data_load_failed(
        &format!("{} has no files to record", fixture.raw().display()),
        "an empty tree is what a killed fetch or a followed junction leaves behind, and a blessing would make it the truth",
        DO_BLESS_EMPTY,
    );

    // Act
    let result = bibex(&fixture, &["raw", "bless"]);

    // Assert
    assert_eq!(result, (Some(EXIT_DATA_LOAD_FAILED), String::new(), expected));
    assert_eq!(fixture.manifest_text(), previous);
}

#[test]
fn raw_bless_refuses_to_overwrite_a_previous_manifest_it_cannot_read() {
    // Arrange
    let fixture = recorded_fixture("bless-unreadable");
    let forged = fixture.manifest_text().replace(FIXTURE_ROOT, ZERO_LOGICAL);
    fs::write(fixture.manifest_path(), &forged).expect("the forged manifest must be writable");
    let expected = format!(
        "atlas: error (integrity_failed): {} does not read, so there is nothing to diff against -- manifest root {ZERO_LOGICAL} does not recompute from its datasets ({FIXTURE_ROOT}) -- {DO_BLESS_UNREADABLE}\n",
        fixture.manifest_path().display()
    );

    // Act
    let result = bibex(&fixture, &["raw", "bless"]);

    // Assert
    assert_eq!(result, (Some(EXIT_INTEGRITY_FAILED), String::new(), expected));
    assert_eq!(fixture.manifest_text(), forged);
}

#[test]
fn raw_bless_refuses_when_there_is_no_raw_tree_to_walk() {
    // Arrange
    let fixture = Fixture::named("bless-no-tree");
    let why = fs::read_dir(fixture.raw()).expect_err("the fixture has no raw tree").to_string();
    let expected = data_load_failed(&format!("{} could not be walked", fixture.raw().display()), &why, DO_BLESS_UNWALKABLE);

    // Act
    let result = bibex(&fixture, &["raw", "bless"]);

    // Assert
    assert_eq!(result, (Some(EXIT_DATA_LOAD_FAILED), String::new(), expected));
    assert!(!fixture.manifest_path().exists());
}

#[test]
fn raw_bless_refuses_when_the_manifest_cannot_be_written() {
    // Arrange
    let fixture = Fixture::named("bless-unwritable");
    fixture.write("geo/ancient.jsonl", ANCIENT);
    fixture.make_dir(MANIFEST_FILE);
    let why = fs::write(fixture.manifest_path(), b"").expect_err("a directory cannot be written as a file").to_string();
    let expected = data_load_failed(&format!("{} could not be written", fixture.manifest_path().display()), &why, DO_BLESS_UNWRITABLE);

    // Act
    let result = bibex(&fixture, &["raw", "bless"]);

    // Assert
    assert_eq!(result, (Some(EXIT_DATA_LOAD_FAILED), String::new(), expected));
}

#[test]
fn raw_without_a_verb_is_bad_usage() {
    // Arrange
    let fixture = Fixture::named("raw-bare");
    let expected = bad_usage("'raw' requires a verb", RAW_USAGE, DO_RAW_VERB);

    // Act
    let result = bibex(&fixture, &["raw"]);

    // Assert
    assert_eq!(result, (Some(EXIT_BAD_USAGE), String::new(), expected));
}

#[test]
fn raw_with_a_verb_it_does_not_know_is_bad_usage() {
    // Arrange
    let fixture = Fixture::named("raw-other");
    let expected = bad_usage("unrecognized arguments for 'raw': curse now", RAW_USAGE, DO_RAW_VERB);

    // Act
    let result = bibex(&fixture, &["raw", "curse", "now"]);

    // Assert
    assert_eq!(result, (Some(EXIT_BAD_USAGE), String::new(), expected));
}

#[test]
fn raw_bless_json_with_a_bad_verb_is_the_same_bad_usage_as_an_envelope() {
    // Arrange
    let fixture = Fixture::named("raw-other-json");
    let expected = json!({ "error": { "code": "bad_usage", "message": format!("'raw' requires a verb -- {RAW_USAGE}"), "hint": DO_RAW_VERB } });

    // Act
    let (code, stdout, stderr) = bibex(&fixture, &["--json", "raw"]);

    // Assert
    assert_eq!((code, stdout, serde_json::from_str::<Value>(&stderr).expect("a JSON envelope on stderr")), (Some(EXIT_BAD_USAGE), String::new(), expected));
}

fn not_found(what: &str, why: &str, do_: &str) -> String {
    format!("atlas: error (not_found): {what} -- {why} -- {do_}\n")
}

fn not_as_recorded(path: &str, differ: &str, failures: &str) -> String {
    format!("atlas: error (integrity_failed): raw {path} is not as recorded ({differ}) -- {failures} -- {DO_RAW}\n")
}

fn walked_hash_of(fixture: &Fixture, dataset: &str) -> String {
    let walked = walk(&fixture.raw()).expect("the fixture tree must walk");
    match walked.datasets.iter().find(|entry| matches!(entry, RawEntry::Node(node) if node.name == dataset)) {
        Some(RawEntry::Node(node)) => node.hash.hex(),
        _ => panic!("the fixture has no directory named {dataset}"),
    }
}

#[test]
fn raw_check_of_a_recorded_file_that_matches_prints_its_hash_and_size() {
    // Arrange
    let fixture = recorded_fixture("check-file");
    let expected = format!("raw geo/ancient.jsonl {ANCIENT_SHA256} OK ({} bytes)\n", ANCIENT.len());

    // Act
    let result = bibex(&fixture, &["raw", "check", "geo/ancient.jsonl"]);

    // Assert
    assert_eq!(result, (Some(EXIT_OK), expected, String::new()));
}

#[test]
fn raw_check_of_a_recorded_directory_that_matches_prints_its_hash_and_file_count() {
    // Arrange
    let fixture = recorded_fixture("check-dir");
    let expected = format!("raw geo {} OK (2 files)\n", walked_hash_of(&fixture, "geo"));

    // Act
    let result = bibex(&fixture, &["raw", "check", "geo"]);

    // Assert
    assert_eq!(result, (Some(EXIT_OK), expected, String::new()));
}

#[test]
fn raw_check_json_carries_the_file_hash_and_size() {
    // Arrange
    let fixture = recorded_fixture("check-file-json");
    let expected = json!({ "path": "geo/ancient.jsonl", "sha256": ANCIENT_SHA256, "bytes": ANCIENT.len() });

    // Act
    let result = bibex_json(&fixture, &["raw", "check", "geo/ancient.jsonl"]);

    // Assert
    assert_eq!(result, (Some(EXIT_OK), expected, String::new()));
}

#[test]
fn raw_check_json_carries_the_directory_hash_and_file_count() {
    // Arrange
    let fixture = recorded_fixture("check-dir-json");
    let expected = json!({ "path": "geo", "hash": walked_hash_of(&fixture, "geo"), "files": 2 });

    // Act
    let result = bibex_json(&fixture, &["raw", "check", "geo"]);

    // Assert
    assert_eq!(result, (Some(EXIT_OK), expected, String::new()));
}

/// Windows only: `\` is a separator there and a name character elsewhere.
#[cfg(windows)]
#[test]
fn raw_check_accepts_the_path_in_windows_spelling_and_answers_in_the_manifests() {
    // Arrange
    let fixture = recorded_fixture("check-backslash");
    let expected = format!("raw geo/ancient.jsonl {ANCIENT_SHA256} OK ({} bytes)\n", ANCIENT.len());

    // Act
    let result = bibex(&fixture, &["raw", "check", "geo\\ancient.jsonl"]);

    // Assert
    assert_eq!(result, (Some(EXIT_OK), expected, String::new()));
}

#[test]
fn raw_check_of_a_truncated_file_exits_6_naming_it_with_both_sizes() {
    // Arrange
    let fixture = recorded_fixture("check-truncated");
    fixture.write("geo/ancient.jsonl", ANCIENT_TRUNCATED);
    let failure = format!("raw geo/ancient.jsonl: TRUNCATED manifest {} bytes file {}", ANCIENT.len(), ANCIENT_TRUNCATED.len());

    // Act
    let result = bibex(&fixture, &["raw", "check", "geo/ancient.jsonl"]);

    // Assert
    assert_eq!(result, (Some(EXIT_INTEGRITY_FAILED), String::new(), not_as_recorded("geo/ancient.jsonl", "1 path differs", &failure)));
}

#[test]
fn raw_check_of_a_flipped_file_names_both_hashes() {
    // Arrange
    let fixture = recorded_fixture("check-flipped");
    fixture.write("geo/ancient.jsonl", ANCIENT_FLIPPED);
    let failure = format!("raw geo/ancient.jsonl: MISMATCH manifest {ANCIENT_SHA256} file {}", hex_of(ANCIENT_FLIPPED));

    // Act
    let result = bibex(&fixture, &["raw", "check", "geo/ancient.jsonl"]);

    // Assert
    assert_eq!(result, (Some(EXIT_INTEGRITY_FAILED), String::new(), not_as_recorded("geo/ancient.jsonl", "1 path differs", &failure)));
}

#[test]
fn raw_check_of_a_missing_file_is_missing() {
    // Arrange
    let fixture = recorded_fixture("check-missing-file");
    fixture.remove("kretzmann/volume-1.txt");

    // Act
    let result = bibex(&fixture, &["raw", "check", "kretzmann/volume-1.txt"]);

    // Assert
    assert_eq!(result, (Some(EXIT_INTEGRITY_FAILED), String::new(), not_as_recorded("kretzmann/volume-1.txt", "1 path differs", "raw kretzmann/volume-1.txt: MISSING")));
}

#[test]
fn raw_check_of_a_missing_directory_is_missing() {
    // Arrange
    let fixture = recorded_fixture("check-missing-dir");
    fixture.remove_dir("kretzmann");

    // Act
    let result = bibex(&fixture, &["raw", "check", "kretzmann"]);

    // Assert
    assert_eq!(result, (Some(EXIT_INTEGRITY_FAILED), String::new(), not_as_recorded("kretzmann", "1 path differs", "raw kretzmann: MISSING")));
}

#[test]
fn raw_check_of_a_half_copied_directory_names_every_file_that_differs() {
    // Arrange
    let fixture = recorded_fixture("check-half-copied");
    fixture.write("geo/ancient.jsonl", ANCIENT_TRUNCATED);
    fixture.remove("geo/modern.jsonl");
    let failures = format!("raw geo/ancient.jsonl: TRUNCATED manifest {} bytes file {}; raw geo/modern.jsonl: MISSING", ANCIENT.len(), ANCIENT_TRUNCATED.len());

    // Act
    let result = bibex(&fixture, &["raw", "check", "geo"]);

    // Assert
    assert_eq!(result, (Some(EXIT_INTEGRITY_FAILED), String::new(), not_as_recorded("geo", "2 paths differ", &failures)));
}

#[test]
fn raw_check_of_an_emptied_directory_names_every_recorded_file_as_missing() {
    // Arrange
    let fixture = recorded_fixture("check-emptied");
    fixture.remove_dir("geo");
    fixture.make_dir("geo");

    // Act
    let result = bibex(&fixture, &["raw", "check", "geo"]);

    // Assert
    assert_eq!(result, (Some(EXIT_INTEGRITY_FAILED), String::new(), not_as_recorded("geo", "2 paths differ", "raw geo/ancient.jsonl: MISSING; raw geo/modern.jsonl: MISSING")));
}

#[test]
fn raw_check_of_a_directory_with_an_extra_file_names_it_as_extra() {
    // Arrange
    let fixture = recorded_fixture("check-extra");
    fixture.write("geo/added.jsonl", ADDED);

    // Act
    let result = bibex(&fixture, &["raw", "check", "geo"]);

    // Assert
    assert_eq!(result, (Some(EXIT_INTEGRITY_FAILED), String::new(), not_as_recorded("geo", "1 path differs", "raw geo/added.jsonl: EXTRA (not in MANIFEST.toml)")));
}

#[test]
fn raw_check_of_a_file_standing_where_a_directory_was_recorded_is_missing_and_extra_at_once() {
    // Arrange
    let fixture = recorded_fixture("check-kind-swap");
    fixture.remove_dir("empty");
    fixture.write("empty", ADDED);

    // Act
    let result = bibex(&fixture, &["raw", "check", "empty"]);

    // Assert
    assert_eq!(result, (Some(EXIT_INTEGRITY_FAILED), String::new(), not_as_recorded("empty", "2 paths differ", "raw empty: MISSING; raw empty: EXTRA (not in MANIFEST.toml)")));
}

#[test]
fn raw_check_of_a_path_the_manifest_does_not_record_is_not_found() {
    // Arrange
    let fixture = recorded_fixture("check-unrecorded");
    fixture.write("geo/added.jsonl", ADDED);
    let expected = not_found("raw geo/added.jsonl is not in MANIFEST.toml", WHY_UNRECORDED, DO_UNRECORDED);

    // Act
    let result = bibex(&fixture, &["raw", "check", "geo/added.jsonl"]);

    // Assert
    assert_eq!(result, (Some(EXIT_NOT_FOUND), String::new(), expected));
}

#[test]
fn raw_check_of_a_path_below_a_recorded_file_is_not_found() {
    // Arrange
    let fixture = recorded_fixture("check-below-file");
    let expected = not_found("raw geo/ancient.jsonl/line is not in MANIFEST.toml", WHY_UNRECORDED, DO_UNRECORDED);

    // Act
    let result = bibex(&fixture, &["raw", "check", "geo/ancient.jsonl/line"]);

    // Assert
    assert_eq!(result, (Some(EXIT_NOT_FOUND), String::new(), expected));
}

#[test]
fn raw_check_with_no_manifest_is_not_found_naming_the_manifest() {
    // Arrange
    let fixture = Fixture::named("check-no-manifest");
    fixture.write("geo/ancient.jsonl", ANCIENT);
    let expected = not_found("raw geo/ancient.jsonl is not in MANIFEST.toml", &format!("no MANIFEST.toml at {}", fixture.manifest_path().display()), DO_UNRECORDED);

    // Act
    let result = bibex(&fixture, &["raw", "check", "geo/ancient.jsonl"]);

    // Assert
    assert_eq!(result, (Some(EXIT_NOT_FOUND), String::new(), expected));
}

#[test]
fn raw_check_against_a_manifest_that_does_not_recompute_is_an_integrity_failure() {
    // Arrange
    let fixture = recorded_fixture("check-bad-manifest");
    let forged = fixture.manifest_text().replace(FIXTURE_ROOT, ZERO_LOGICAL);
    fs::write(fixture.manifest_path(), forged).expect("the forged manifest must be writable");
    let expected = format!(
        "atlas: error (integrity_failed): {} does not verify -- manifest root {ZERO_LOGICAL} does not recompute from its datasets ({FIXTURE_ROOT}) -- {DO_RAW}\n",
        fixture.manifest_path().display()
    );

    // Act
    let result = bibex(&fixture, &["raw", "check", "geo"]);

    // Assert
    assert_eq!(result, (Some(EXIT_INTEGRITY_FAILED), String::new(), expected));
}

#[cfg(windows)]
#[test]
fn raw_check_of_a_junction_standing_where_a_directory_was_recorded_is_missing_and_a_link() {
    // Arrange
    let fixture = recorded_fixture("check-junction-in-place");
    fixture.remove_dir("geo");
    let link = fixture.junction("geo");
    let failures = "raw geo: MISSING; raw geo: LINK (a junction or symlink MANIFEST.toml does not list)";

    // Act
    let result = bibex(&fixture, &["raw", "check", "geo"]);
    fs::remove_dir(&link).expect("the junction must be removable on its own");

    // Assert
    assert_eq!(result, (Some(EXIT_INTEGRITY_FAILED), String::new(), not_as_recorded("geo", "2 paths differ", failures)));
}

#[cfg(windows)]
#[test]
fn raw_check_of_a_directory_holding_an_unlisted_junction_names_the_link() {
    // Arrange
    let fixture = recorded_fixture("check-junction-inside");
    let link = fixture.junction("geo/linked");

    // Act
    let result = bibex(&fixture, &["raw", "check", "geo"]);
    fs::remove_dir(&link).expect("the junction must be removable on its own");

    // Assert
    assert_eq!(result, (Some(EXIT_INTEGRITY_FAILED), String::new(), not_as_recorded("geo", "1 path differs", "raw geo/linked: LINK (a junction or symlink MANIFEST.toml does not list)")));
}

#[cfg(windows)]
#[test]
fn raw_check_of_a_directory_whose_listed_junction_is_gone_names_the_missing_link() {
    // Arrange
    let fixture = recorded_fixture("check-junction-gone");
    let link = fixture.junction("geo/linked");
    fixture.record();
    fs::remove_dir(&link).expect("the junction must be removable on its own");

    // Act
    let result = bibex(&fixture, &["raw", "check", "geo"]);

    // Assert
    assert_eq!(result, (Some(EXIT_INTEGRITY_FAILED), String::new(), not_as_recorded("geo", "1 path differs", "raw geo/linked: LINK MISSING (MANIFEST.toml lists a junction or symlink there)")));
}

/// Windows only: an exclusive open (share mode 0) is the one portable way this test can make a
/// file unreadable without elevation; other hosts have no share modes.
#[cfg(windows)]
#[test]
fn raw_check_of_a_file_it_cannot_read_names_the_file() {
    // Arrange
    use std::os::windows::fs::OpenOptionsExt;
    let fixture = recorded_fixture("check-unreadable");
    let locked = fixture.raw().join("geo").join("ancient.jsonl");
    let _hold = fs::OpenOptions::new().read(true).share_mode(0).open(&locked).expect("the fixture file must open exclusively");
    let why = fs::read(&locked).expect_err("an exclusively held file must not read").to_string();
    let expected = format!("atlas: error (integrity_failed): raw geo/ancient.jsonl could not be read -- {}: {why} -- {DO_RAW}\n", locked.display());

    // Act
    let result = bibex(&fixture, &["raw", "check", "geo/ancient.jsonl"]);

    // Assert
    assert_eq!(result, (Some(EXIT_INTEGRITY_FAILED), String::new(), expected));
}

#[test]
fn raw_check_without_a_path_is_bad_usage() {
    // Arrange
    let fixture = Fixture::named("check-no-path");
    let expected = bad_usage("'raw check' requires a <path> argument", RAW_USAGE, DO_RAW_VERB);

    // Act
    let result = bibex(&fixture, &["raw", "check"]);

    // Assert
    assert_eq!(result, (Some(EXIT_BAD_USAGE), String::new(), expected));
}

#[test]
fn raw_check_with_more_than_a_path_is_bad_usage() {
    // Arrange
    let fixture = Fixture::named("check-two-paths");
    let expected = bad_usage("unrecognized arguments for 'raw': check geo kretzmann", RAW_USAGE, DO_RAW_VERB);

    // Act
    let result = bibex(&fixture, &["raw", "check", "geo", "kretzmann"]);

    // Assert
    assert_eq!(result, (Some(EXIT_BAD_USAGE), String::new(), expected));
}

#[test]
fn raw_check_of_a_path_that_climbs_out_of_data_raw_is_bad_usage() {
    // Arrange
    let fixture = recorded_fixture("check-climbs");
    let expected = bad_usage("'../kjv.json' is not a path under data/raw", WHY_BAD_PATH, DO_BAD_PATH);

    // Act
    let result = bibex(&fixture, &["raw", "check", "../kjv.json"]);

    // Assert
    assert_eq!(result, (Some(EXIT_BAD_USAGE), String::new(), expected));
}

#[test]
fn raw_check_of_an_empty_path_is_bad_usage() {
    // Arrange
    let fixture = recorded_fixture("check-empty-path");
    let expected = bad_usage("'' is not a path under data/raw", WHY_BAD_PATH, DO_BAD_PATH);

    // Act
    let result = bibex(&fixture, &["raw", "check", ""]);

    // Assert
    assert_eq!(result, (Some(EXIT_BAD_USAGE), String::new(), expected));
}
