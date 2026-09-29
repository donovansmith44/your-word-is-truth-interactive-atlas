use std::fs;
use std::io;
use std::path::PathBuf;

use atlas_cli::raw::{read_manifest, walk, write_manifest, ManifestError, RawManifest, MANIFEST_FILE, MANIFEST_SCHEMA};
use atlas_graph_types::raw_manifest::{node_hash, HexError, RawEntry, RawHash, RawLeaf, RawNode, Sha256};
use atlas_graph_types::sha256::sha256;

const ANCIENT: &[u8] = b"ancient places\n";
const MODERN: &[u8] = b"modern places\n";
const VOLUME_ONE: &[u8] = b"kretzmann volume one\n";
const KJV: &[u8] = b"kjv\n";
const ENTRIES: &[u8] = b"lexicon entries\n";
const ELSEWHERE: &[u8] = b"a file the walker must never reach\n";
const ANCIENT_FLIPPED: &[u8] = b"Ancient places\n";
const ADDED: &[u8] = b"added\n";
const NO_LINKS: Vec<String> = Vec::new();

struct Fixture {
    dir: PathBuf,
}

impl Fixture {
    fn named(test: &str) -> Fixture {
        let dir = std::env::temp_dir().join(format!("raw-walk-{}-{test}", std::process::id()));
        fs::create_dir_all(&dir).expect("the fixture directory must be creatable");
        Fixture { dir }
    }

    fn root(&self) -> PathBuf {
        self.dir.join("raw")
    }

    fn write(&self, relative: &str, bytes: &[u8]) {
        let path = self.root().join(relative);
        fs::create_dir_all(path.parent().expect("every fixture file sits in a directory")).expect("fixture directories must be creatable");
        fs::write(path, bytes).expect("fixture files must be writable");
    }

    fn make_dir(&self, relative: &str) {
        fs::create_dir_all(self.root().join(relative)).expect("fixture directories must be creatable");
    }

    fn remove(&self, relative: &str) {
        fs::remove_file(self.root().join(relative)).expect("fixture files must be removable");
    }

    fn manifest_path(&self) -> PathBuf {
        self.dir.join(MANIFEST_FILE)
    }

    fn write_manifest_text(&self, text: &str) -> PathBuf {
        let path = self.manifest_path();
        fs::write(&path, text).expect("the manifest text must be writable");
        path
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

fn three_files_in_two_directories_and_one_empty(test: &str) -> Fixture {
    let fixture = Fixture::named(test);
    fixture.write("kretzmann/volume-1.txt", VOLUME_ONE);
    fixture.write("geo/modern.jsonl", MODERN);
    fixture.write("geo/ancient.jsonl", ANCIENT);
    fixture.make_dir("empty");
    fixture
}

fn leaf(name: &str, bytes: &[u8]) -> RawEntry {
    RawEntry::Leaf(RawLeaf { name: name.into(), sha256: Sha256(sha256(bytes)), bytes: bytes.len() as u64 })
}

fn node(name: &str, children: Vec<RawEntry>) -> RawEntry {
    RawEntry::Node(RawNode { name: name.into(), hash: node_hash(&children), children })
}

fn manifest_over(datasets: Vec<RawEntry>, links: Vec<String>) -> RawManifest {
    RawManifest { schema: MANIFEST_SCHEMA, root: node_hash(&datasets), datasets, links }
}

fn hand_computed_fixture_manifest() -> RawManifest {
    manifest_over(
        vec![
            node("empty", vec![]),
            node("geo", vec![leaf("ancient.jsonl", ANCIENT), leaf("modern.jsonl", MODERN)]),
            node("kretzmann", vec![leaf("volume-1.txt", VOLUME_ONE)]),
        ],
        NO_LINKS,
    )
}

fn dataset<'a>(manifest: &'a RawManifest, name: &str) -> &'a RawNode {
    manifest
        .datasets
        .iter()
        .find_map(|entry| match entry {
            RawEntry::Node(node) if node.name == name => Some(node),
            _ => None,
        })
        .unwrap_or_else(|| panic!("dataset {name} must be a directory of the manifest"))
}

fn file_of<'a>(node: &'a RawNode, name: &str) -> &'a RawLeaf {
    node.children
        .iter()
        .find_map(|entry| match entry {
            RawEntry::Leaf(leaf) if leaf.name == name => Some(leaf),
            _ => None,
        })
        .unwrap_or_else(|| panic!("{name} must be a file of {}", node.name))
}

fn hex_of_file(bytes: &[u8]) -> String {
    Sha256(sha256(bytes)).hex()
}

fn hex_of_node(manifest: &RawManifest, name: &str) -> String {
    dataset(manifest, name).hash.hex()
}

#[test]
fn walk_builds_the_hand_computed_tree_over_the_fixture() {
    // Arrange
    let fixture = three_files_in_two_directories_and_one_empty("hand-computed");

    // Act
    let walked = walk(&fixture.root()).expect("the fixture must walk");

    // Assert
    assert_eq!(walked, hand_computed_fixture_manifest());
}

#[test]
fn a_flipped_byte_moves_the_leaf_its_directory_and_the_root_and_no_sibling() {
    // Arrange
    let fixture = three_files_in_two_directories_and_one_empty("flipped-byte");
    let before = walk(&fixture.root()).expect("the fixture must walk");
    fixture.write("geo/ancient.jsonl", ANCIENT_FLIPPED);

    // Act
    let after = walk(&fixture.root()).expect("the flipped fixture must walk");

    // Assert
    assert_ne!(file_of(dataset(&after, "geo"), "ancient.jsonl").sha256, file_of(dataset(&before, "geo"), "ancient.jsonl").sha256);
    assert_ne!(dataset(&after, "geo").hash, dataset(&before, "geo").hash);
    assert_ne!(after.root, before.root);
    assert_eq!(file_of(dataset(&after, "geo"), "modern.jsonl"), file_of(dataset(&before, "geo"), "modern.jsonl"));
    assert_eq!(dataset(&after, "kretzmann"), dataset(&before, "kretzmann"));
    assert_eq!(dataset(&after, "empty"), dataset(&before, "empty"));
}

#[test]
fn a_file_added_moves_the_root() {
    // Arrange
    let fixture = three_files_in_two_directories_and_one_empty("added");
    let before = walk(&fixture.root()).expect("the fixture must walk");
    fixture.write("kretzmann/volume-2.txt", ADDED);

    // Act
    let after = walk(&fixture.root()).expect("the grown fixture must walk");

    // Assert
    assert_ne!(after.root, before.root);
    assert_eq!(
        after,
        manifest_over(
            vec![
                node("empty", vec![]),
                node("geo", vec![leaf("ancient.jsonl", ANCIENT), leaf("modern.jsonl", MODERN)]),
                node("kretzmann", vec![leaf("volume-1.txt", VOLUME_ONE), leaf("volume-2.txt", ADDED)]),
            ],
            NO_LINKS,
        )
    );
}

#[test]
fn a_file_removed_moves_the_root() {
    // Arrange
    let fixture = three_files_in_two_directories_and_one_empty("removed");
    let before = walk(&fixture.root()).expect("the fixture must walk");
    fixture.remove("geo/modern.jsonl");

    // Act
    let after = walk(&fixture.root()).expect("the shrunk fixture must walk");

    // Assert
    assert_ne!(after.root, before.root);
    assert_eq!(
        after,
        manifest_over(
            vec![
                node("empty", vec![]),
                node("geo", vec![leaf("ancient.jsonl", ANCIENT)]),
                node("kretzmann", vec![leaf("volume-1.txt", VOLUME_ONE)]),
            ],
            NO_LINKS,
        )
    );
}

#[test]
fn two_walks_agree_and_every_level_is_name_ordered_whatever_order_the_files_were_made_in() {
    // Arrange
    let fixture = Fixture::named("deterministic");
    fixture.write("z/z.txt", ADDED);
    fixture.write("z/a.txt", ADDED);
    fixture.write("a.txt", KJV);
    fixture.make_dir("z/m");
    fixture.write("z/b.txt", ADDED);
    fixture.make_dir("b");

    // Act
    let first = walk(&fixture.root()).expect("the fixture must walk");
    let second = walk(&fixture.root()).expect("the fixture must walk again");

    // Assert
    assert_eq!(first, second);
    assert_eq!(
        first,
        manifest_over(
            vec![
                leaf("a.txt", KJV),
                node("b", vec![]),
                node("z", vec![leaf("a.txt", ADDED), leaf("b.txt", ADDED), node("m", vec![]), leaf("z.txt", ADDED)]),
            ],
            NO_LINKS,
        )
    );
}

#[cfg(windows)]
#[test]
fn a_junction_is_listed_as_a_link_and_never_followed_into_its_target() {
    // Arrange
    let fixture = three_files_in_two_directories_and_one_empty("junction");
    let elsewhere = fixture.dir.join("elsewhere");
    fs::create_dir_all(&elsewhere).expect("the junction target must be creatable");
    fs::write(elsewhere.join("secret.txt"), ELSEWHERE).expect("the target's file must be writable");
    let link = fixture.root().join("geo").join("linked");
    let made = std::process::Command::new("cmd")
        .args(["/C", "mklink", "/J", link.to_str().expect("utf-8 path"), elsewhere.to_str().expect("utf-8 path")])
        .output()
        .expect("cmd must run");
    assert!(made.status.success(), "mklink /J: {}", String::from_utf8_lossy(&made.stderr));

    // Act
    let walked = walk(&fixture.root()).expect("a tree with a junction must still walk");
    let written = write_manifest(&fixture.manifest_path(), &walked).and_then(|()| fs::read_to_string(fixture.manifest_path()));
    fs::remove_dir(&link).expect("the junction must be removable on its own");

    // Assert
    let mut expected = hand_computed_fixture_manifest();
    expected.links = vec!["geo/linked".into()];
    assert_eq!(walked, expected);
    assert_eq!(
        written.expect("the manifest must be written and read back"),
        format!(
            "schema = {MANIFEST_SCHEMA}\nroot = \"{}\"\nlinks = [\"geo/linked\"]\n\n[[dataset]]\nname = \"empty\"\nhash = \"{}\"\n\n[[dataset]]\nname = \"geo\"\nhash = \"{}\"\n\n[[dataset.file]]\nname = \"ancient.jsonl\"\nsha256 = \"{}\"\nbytes = {}\n\n[[dataset.file]]\nname = \"modern.jsonl\"\nsha256 = \"{}\"\nbytes = {}\n\n[[dataset]]\nname = \"kretzmann\"\nhash = \"{}\"\n\n[[dataset.file]]\nname = \"volume-1.txt\"\nsha256 = \"{}\"\nbytes = {}\n",
            expected.root.hex(),
            hex_of_node(&expected, "empty"),
            hex_of_node(&expected, "geo"),
            hex_of_file(ANCIENT),
            ANCIENT.len(),
            hex_of_file(MODERN),
            MODERN.len(),
            hex_of_node(&expected, "kretzmann"),
            hex_of_file(VOLUME_ONE),
            VOLUME_ONE.len(),
        )
    );
    assert!(elsewhere.join("secret.txt").exists(), "the target outlives the walk and the junction's removal");
}

#[test]
fn the_readme_and_the_manifest_itself_are_kept_by_git_and_so_are_not_leaves() {
    // Arrange
    let fixture = three_files_in_two_directories_and_one_empty("kept-by-git");
    fixture.write("README.md", KJV);
    fixture.write(MANIFEST_FILE, KJV);
    fixture.write("geo/README.md", KJV);

    // Act
    let walked = walk(&fixture.root()).expect("the fixture must walk");

    // Assert
    assert_eq!(
        walked,
        manifest_over(
            vec![
                node("empty", vec![]),
                node("geo", vec![leaf("README.md", KJV), leaf("ancient.jsonl", ANCIENT), leaf("modern.jsonl", MODERN)]),
                node("kretzmann", vec![leaf("volume-1.txt", VOLUME_ONE)]),
            ],
            NO_LINKS,
        )
    );
}

#[cfg(windows)]
#[test]
fn a_name_holding_a_lone_surrogate_the_manifest_cannot_spell_is_refused() {
    // Arrange
    use std::os::windows::ffi::OsStringExt;
    let fixture = three_files_in_two_directories_and_one_empty("unspellable");
    let lone_surrogate = std::ffi::OsString::from_wide(&[0xD800]);
    fs::write(fixture.root().join("geo").join(lone_surrogate), KJV).expect("NTFS accepts a lone surrogate");

    // Act
    let walked = walk(&fixture.root());

    // Assert
    let error = walked.expect_err("an unspellable name must refuse the walk");
    assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    assert_eq!(error.to_string(), format!("{}: a name the manifest cannot spell: \"\\u{{d800}}\"", fixture.root().join("geo").display()));
}

#[test]
fn a_missing_tree_is_an_io_error() {
    // Arrange
    let fixture = Fixture::named("missing-tree");

    // Act
    let walked = walk(&fixture.root());

    // Assert
    assert_eq!(walked.expect_err("a missing tree cannot be walked").kind(), io::ErrorKind::NotFound);
}

#[test]
fn write_then_read_round_trips_a_tree_with_a_top_level_file_and_a_nested_directory_and_the_toml_is_exactly_this() {
    // Arrange
    let fixture = Fixture::named("round-trip");
    fixture.write("kjv.json", KJV);
    fixture.write("brain-fuel-bible/lexicon/entries.txt", ENTRIES);
    fixture.write("brain-fuel-bible/readme.txt", KJV);
    let walked = walk(&fixture.root()).expect("the fixture must walk");

    // Act
    write_manifest(&fixture.manifest_path(), &walked).expect("the manifest must be writable");
    let text = fs::read_to_string(fixture.manifest_path()).expect("the manifest must be readable");
    let read = read_manifest(&fixture.manifest_path()).expect("the written manifest must read back");

    // Assert
    let lexicon = node("lexicon", vec![leaf("entries.txt", ENTRIES)]);
    let brain_fuel = node("brain-fuel-bible", vec![lexicon.clone(), leaf("readme.txt", KJV)]);
    let expected = manifest_over(vec![brain_fuel.clone(), leaf("kjv.json", KJV)], NO_LINKS);
    assert_eq!(walked, expected);
    assert_eq!(read, expected);
    let RawEntry::Node(brain_fuel) = brain_fuel else { unreachable!() };
    let RawEntry::Node(lexicon) = lexicon else { unreachable!() };
    assert_eq!(
        text,
        format!(
            "schema = {MANIFEST_SCHEMA}\nroot = \"{}\"\n\n[[dataset]]\nname = \"brain-fuel-bible\"\nhash = \"{}\"\n\n[[dataset.file]]\nname = \"readme.txt\"\nsha256 = \"{}\"\nbytes = {}\n\n[[dataset.dir]]\nname = \"lexicon\"\nhash = \"{}\"\n\n[[dataset.dir.file]]\nname = \"entries.txt\"\nsha256 = \"{}\"\nbytes = {}\n\n[[dataset]]\nname = \"kjv.json\"\nsha256 = \"{}\"\nbytes = {}\n",
            expected.root.hex(),
            brain_fuel.hash.hex(),
            hex_of_file(KJV),
            KJV.len(),
            lexicon.hash.hex(),
            hex_of_file(ENTRIES),
            ENTRIES.len(),
            hex_of_file(KJV),
            KJV.len(),
        )
    );
}

#[test]
fn an_empty_tree_writes_its_schema_and_root_and_nothing_else() {
    // Arrange
    let fixture = Fixture::named("empty-tree");
    fixture.make_dir("");
    let walked = walk(&fixture.root()).expect("an empty tree walks");

    // Act
    write_manifest(&fixture.manifest_path(), &walked).expect("the manifest must be writable");
    let text = fs::read_to_string(fixture.manifest_path()).expect("the manifest must be readable");

    // Assert
    assert_eq!(walked, manifest_over(vec![], NO_LINKS));
    assert_eq!(text, format!("schema = {MANIFEST_SCHEMA}
root = \"{}\"
", node_hash(&[]).hex()));
}

#[test]
fn read_refuses_a_top_level_key_it_does_not_know() {
    // Arrange
    let fixture = Fixture::named("read-unknown-key");
    let path = fixture.write_manifest_text(&format!("schema = 1
root = \"{}\"
blessed = \"2026-09-29\"
", node_hash(&[]).hex()));

    // Act
    let read = read_manifest(&path);

    // Assert
    let error = read.expect_err("a key the schema does not declare is refused");
    assert!(matches!(error, ManifestError::Toml(_)), "{error:?}");
    assert_eq!(
        error.to_string(),
        "manifest parse: TOML parse error at line 3, column 1
  |
3 | blessed = \"2026-09-29\"
  | ^^^^^^^
unknown field `blessed`, expected one of `schema`, `root`, `links`, `dataset`
"
    );
}

#[test]
fn read_refuses_a_file_row_with_a_key_it_does_not_know() {
    // Arrange
    let fixture = Fixture::named("read-unknown-row-key");
    let path = fixture.write_manifest_text(&format!(
        "schema = 1
root = \"{}\"

[[dataset]]
name = \"kjv.json\"
sha256 = \"{}\"
bytes = {}
kind = \"json\"
",
        node_hash(&[leaf("kjv.json", KJV)]).hex(),
        hex_of_file(KJV),
        KJV.len(),
    ));

    // Act
    let read = read_manifest(&path);

    // Assert
    let error = read.expect_err("a row key the schema does not declare is refused");
    assert!(matches!(error, ManifestError::Toml(_)), "{error:?}");
    assert_eq!(
        error.to_string(),
        "manifest parse: TOML parse error at line 4, column 1
  |
4 | [[dataset]]
  | ^^^^^^^^^^^
data did not match any variant of untagged enum EntryRow
"
    );
}

#[test]
fn read_refuses_a_missing_manifest_as_not_found() {
    // Arrange
    let fixture = Fixture::named("read-missing");

    // Act
    let read = read_manifest(&fixture.manifest_path());

    // Assert
    match read {
        Err(ManifestError::Io(error)) => assert_eq!(error.kind(), io::ErrorKind::NotFound),
        other => panic!("a missing manifest must be an io error, got {other:?}"),
    }
}

#[test]
fn read_refuses_text_that_is_not_the_manifest_shape() {
    // Arrange
    let fixture = Fixture::named("read-malformed");
    let path = fixture.write_manifest_text("schema = 1\nroot = \"00\"\n[[dataset]]\nname = \"geo\"\n");

    // Act
    let read = read_manifest(&path);

    // Assert
    let error = read.expect_err("a dataset that is neither a file nor a directory is refused");
    assert!(matches!(error, ManifestError::Toml(_)), "{error:?}");
    assert_eq!(
        error.to_string(),
        "manifest parse: TOML parse error at line 3, column 1\n  |\n3 | [[dataset]]\n  | ^^^^^^^^^^^\ndata did not match any variant of untagged enum EntryRow\n"
    );
}

#[test]
fn read_refuses_another_schema() {
    // Arrange
    let fixture = Fixture::named("read-schema");
    let path = fixture.write_manifest_text(&format!("schema = 2\nroot = \"{}\"\n", node_hash(&[]).hex()));

    // Act
    let read = read_manifest(&path);

    // Assert
    let error = read.expect_err("schema 2 is not this reader's schema");
    assert!(matches!(error, ManifestError::Schema { found: 2 }), "{error:?}");
    assert_eq!(error.to_string(), "manifest schema 2 is not 1");
}

#[test]
fn read_refuses_a_hash_that_is_not_hex_naming_the_entry() {
    // Arrange
    let fixture = Fixture::named("read-hex");
    let path = fixture.write_manifest_text(&format!(
        "schema = 1\nroot = \"{}\"\n\n[[dataset]]\nname = \"geo\"\nhash = \"{}\"\n\n[[dataset.file]]\nname = \"ancient.jsonl\"\nsha256 = \"{}\"\nbytes = 3\n",
        node_hash(&[]).hex(),
        node_hash(&[]).hex(),
        "0".repeat(63),
    ));

    // Act
    let read = read_manifest(&path);

    // Assert
    let error = read.expect_err("63 digits are not a sha256");
    assert!(matches!(&error, ManifestError::Hex { path, error: HexError::Length { expected: 64, found: 63 } } if path == "geo/ancient.jsonl"), "{error:?}");
    assert_eq!(error.to_string(), "manifest geo/ancient.jsonl: expected 64 hex digits, found 63");
}

#[test]
fn read_refuses_a_root_that_is_not_hex_naming_the_root_key() {
    // Arrange
    let fixture = Fixture::named("read-root-hex");
    let path = fixture.write_manifest_text("schema = 1
root = \"00\"
");

    // Act
    let read = read_manifest(&path);

    // Assert
    let error = read.expect_err("two digits are not a raw hash");
    assert!(matches!(&error, ManifestError::Hex { path, error: HexError::Length { expected: 32, found: 2 } } if path == "root"), "{error:?}");
    assert_eq!(error.to_string(), "manifest root: expected 32 hex digits, found 2");
}

#[test]
fn read_refuses_a_hash_digit_that_is_not_hex_naming_the_entry_and_the_position() {
    // Arrange
    let fixture = Fixture::named("read-digit");
    let mut bad = node_hash(&[]).hex();
    bad.replace_range(2..3, "X");
    let path = fixture.write_manifest_text(&format!("schema = 1\nroot = \"{}\"\n\n[[dataset]]\nname = \"empty\"\nhash = \"{bad}\"\n", node_hash(&[]).hex()));

    // Act
    let read = read_manifest(&path);

    // Assert
    let error = read.expect_err("X is not a hex digit");
    assert!(matches!(&error, ManifestError::Hex { path, error: HexError::Digit { at: 2, found: 'X' } } if path == "empty"), "{error:?}");
    assert_eq!(error.to_string(), "manifest empty: 'X' at 2 is not a lowercase hex digit");
}

#[test]
fn read_refuses_a_directory_whose_hash_does_not_recompute_from_its_children() {
    // Arrange
    let fixture = Fixture::named("read-node");
    let recorded = RawHash([0x11; 16]);
    let empty = node("empty", vec![]);
    let path = fixture.write_manifest_text(&format!(
        "schema = 1\nroot = \"{}\"\n\n[[dataset]]\nname = \"empty\"\nhash = \"{}\"\n",
        node_hash(&[empty]).hex(),
        recorded.hex(),
    ));

    // Act
    let read = read_manifest(&path);

    // Assert
    let error = read.expect_err("a directory hash that does not recompute is refused");
    let recomputed = node_hash(&[]);
    assert!(matches!(&error, ManifestError::Node { path, recorded: r, recomputed: c } if path == "empty" && *r == recorded && *c == recomputed), "{error:?}");
    assert_eq!(error.to_string(), format!("manifest empty: hash {} does not recompute from its children ({})", recorded.hex(), recomputed.hex()));
}

#[test]
fn read_refuses_a_root_that_does_not_recompute_from_its_datasets() {
    // Arrange
    let fixture = Fixture::named("read-root");
    let recorded = RawHash([0x22; 16]);
    let path = fixture.write_manifest_text(&format!("schema = 1\nroot = \"{}\"\n\n[[dataset]]\nname = \"empty\"\nhash = \"{}\"\n", recorded.hex(), node_hash(&[]).hex()));

    // Act
    let read = read_manifest(&path);

    // Assert
    let error = read.expect_err("a root that does not recompute is refused");
    let recomputed = node_hash(&[node("empty", vec![])]);
    assert!(matches!(&error, ManifestError::Root { recorded: r, recomputed: c } if *r == recorded && *c == recomputed), "{error:?}");
    assert_eq!(error.to_string(), format!("manifest root {} does not recompute from its datasets ({})", recorded.hex(), recomputed.hex()));
}

#[test]
fn read_orders_datasets_and_children_by_name_whatever_order_the_file_lists_them_in() {
    // Arrange
    let fixture = Fixture::named("read-order");
    let geo = node("geo", vec![leaf("ancient.jsonl", ANCIENT), node("m", vec![]), leaf("modern.jsonl", MODERN)]);
    let RawEntry::Node(geo_node) = &geo else { unreachable!() };
    let expected = manifest_over(vec![node("empty", vec![]), geo.clone(), leaf("kjv.json", KJV)], NO_LINKS);
    let path = fixture.write_manifest_text(&format!(
        "schema = 1\nroot = \"{}\"\n\n[[dataset]]\nname = \"kjv.json\"\nsha256 = \"{}\"\nbytes = {}\n\n[[dataset]]\nname = \"geo\"\nhash = \"{}\"\n\n[[dataset.dir]]\nname = \"m\"\nhash = \"{}\"\n\n[[dataset.file]]\nname = \"modern.jsonl\"\nsha256 = \"{}\"\nbytes = {}\n\n[[dataset.file]]\nname = \"ancient.jsonl\"\nsha256 = \"{}\"\nbytes = {}\n\n[[dataset]]\nname = \"empty\"\nhash = \"{}\"\n",
        expected.root.hex(),
        hex_of_file(KJV),
        KJV.len(),
        geo_node.hash.hex(),
        node_hash(&[]).hex(),
        hex_of_file(MODERN),
        MODERN.len(),
        hex_of_file(ANCIENT),
        ANCIENT.len(),
        node_hash(&[]).hex(),
    ));

    // Act
    let read = read_manifest(&path).expect("a manifest in another order still reads");

    // Assert
    assert_eq!(read, expected);
}

#[test]
fn read_reads_back_the_links_a_walk_refused_to_follow() {
    // Arrange
    let fixture = Fixture::named("read-links");
    let path = fixture.write_manifest_text(&format!("schema = 1\nroot = \"{}\"\nlinks = [\"geo/linked\", \"theographic\"]\n", node_hash(&[]).hex()));

    // Act
    let read = read_manifest(&path).expect("a manifest with links reads");

    // Assert
    assert_eq!(read, manifest_over(vec![], vec!["geo/linked".into(), "theographic".into()]));
}

#[test]
fn write_refuses_a_path_whose_directory_does_not_exist() {
    // Arrange
    let fixture = Fixture::named("write-missing-dir");
    let path = fixture.dir.join("nowhere").join(MANIFEST_FILE);

    // Act
    let written = write_manifest(&path, &manifest_over(vec![], NO_LINKS));

    // Assert
    assert_eq!(written.expect_err("a missing directory cannot hold the manifest").kind(), io::ErrorKind::NotFound);
}
